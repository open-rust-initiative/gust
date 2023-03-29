//!
//!
//!
//!
use std::collections::{HashMap, HashSet};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use std::str::FromStr;

use anyhow::Result;
use axum::body::Body;
use axum::http::{Response, StatusCode};
use bstr::ByteSlice;
use bytes::{Buf, BufMut, Bytes, BytesMut};
use chrono::Utc;
use futures::StreamExt;
use hyper::Request;

use crate::git::hash::Hash;
use crate::git::object::base::blob::Blob;
use crate::git::object::base::commit::Commit;
use crate::git::object::base::tree::{Tree, TreeItemType};
use crate::git::object::metadata::MetaData;
use crate::git::pack::Pack;
use crate::git::protocol::{http, PackProtocol, RefCommand};
use crate::gust::driver::ObjectStorage;

use super::{Capability, Protocol, ServiceType, SideBind};

const LF: char = '\n';

pub const SP: char = ' ';

const NUL: char = '\0';

pub const PKT_LINE_END_MARKER: &[u8; 4] = b"0000";

// The atomic, report-status, report-status-v2, delete-refs, quiet,
// and push-cert capabilities are sent and recognized by the receive-pack (push to server) process.
const RECEIVE_CAP_LIST: &str = "report-status report-status-v2 delete-refs quiet atomic ";

// The ofs-delta and side-band-64k capabilities are sent and recognized by both upload-pack and receive-pack protocols.
// The agent and session-id capabilities may optionally be sent in both protocols.
const CAP_LIST: &str = "side-band-64k ofs-delta object-format=sha1";

// All other capabilities are only recognized by the upload-pack (fetch from server) process.
const UPLOAD_CAP_LIST: &str =
    "shallow deepen-since deepen-not deepen-relative multi_ack_detailed no-done ";

impl<T: ObjectStorage> PackProtocol<T> {
    pub async fn git_info_refs(&mut self) -> BytesMut {
        let service_type = self.service_type.unwrap();
        // use zero_id if init_repo
        let zero_id = String::from_utf8_lossy(&[b'0'; 40]).to_string();
        // The stream MUST include capability declarations behind a NUL on the first ref.
        let object_id = self.storage.get_head_object_id(&self.path).await;
        let name = if object_id == zero_id {
            "capabilities^{}"
        } else {
            "HEAD"
        };
        let cap_list = match self.service_type {
            Some(ServiceType::UploadPack) => format!("{}{}", UPLOAD_CAP_LIST, CAP_LIST),
            Some(ServiceType::ReceivePack) => format!("{}{}", RECEIVE_CAP_LIST, CAP_LIST),
            _ => CAP_LIST.to_owned(),
        };
        let pkt_line = format!("{}{}{}{}{}{}", object_id, SP, name, NUL, cap_list, LF);
        let mut ref_list = vec![pkt_line];

        let obj_ids = self.storage.get_ref_object_id(&self.path).await;
        for (object_id, name) in obj_ids {
            let pkt_line = format!("{}{}{}{}", object_id, SP, name, LF);
            ref_list.push(pkt_line);
        }
        let pkt_line_stream = self.build_smart_reply(&ref_list, service_type.to_string());
        tracing::info!("git_info_refs response: {:?}", pkt_line_stream);
        pkt_line_stream
    }

    pub async fn git_upload_pack(
        &mut self,
        upload_request: &mut Bytes,
    ) -> Result<(Vec<u8>, BytesMut)> {
        let mut want: Vec<String> = Vec::new();
        let mut have: Vec<String> = Vec::new();

        let mut first_line = true;
        loop {
            let (bytes_take, pkt_line) = read_pkt_line(upload_request);
            // if read 0000
            if bytes_take == 0 {
                if pkt_line.is_empty() {
                    break;
                }
            }
            tracing::info!("read line: {:?}", pkt_line);
            let dst = pkt_line.to_vec();
            let commands = &dst[0..4];

            match commands {
                b"want" => want.push(String::from_utf8(dst[5..45].to_vec()).unwrap()),
                b"have" => have.push(String::from_utf8(dst[5..45].to_vec()).unwrap()),
                b"done" => break,
                other => {
                    println!(
                        "unsupported command: {:?}",
                        String::from_utf8(other.to_vec())
                    );
                    continue;
                }
            }
            if first_line {
                self.parse_capabilities(&String::from_utf8(dst[46..].to_vec()).unwrap());
                first_line = false;
            }
        }

        tracing::info!(
            "want commands: {:?}, have commans: {:?}, caps:{:?}",
            want,
            have,
            self.capabilities
        );

        let send_pack_data;
        let mut buf = BytesMut::new();

        if have.is_empty() {
            send_pack_data = self.storage.get_full_pack_data(&self.path).await;
            add_pkt_line_string(&mut buf, String::from("NAK\n"));
        } else {
            let object_root = self.path.join(".git/objects");
            // TODO storage.handle_pull_pack_data(&self.path).await;
            let mut decoded_pack = Pack::default();
            let meta_map: HashMap<Hash, MetaData> =
                find_common_base(Hash::from_str(&want[0]).unwrap(), &object_root, &have);
            send_pack_data = decoded_pack.encode(Some(meta_map.into_values().collect()));

            // multi_ack_detailed mode, the server will differentiate the ACKs where it is signaling that
            // it is ready to send data with ACK obj-id ready lines,
            // and signals the identified common commits with ACK obj-id common lines
            for commit in &have {
                add_pkt_line_string(&mut buf, format!("ACK {} common\n", commit));
            }
            // If multi_ack_detailed and no-done are both present, then the sender is free to immediately send a pack
            // following its first "ACK obj-id ready" message.
            add_pkt_line_string(&mut buf, format!("ACK {} ready\n", have[have.len() - 1]));

            add_pkt_line_string(&mut buf, format!("ACK {} \n", have[have.len() - 1]));
        }
        Ok((send_pack_data, buf))
    }

    pub async fn git_receive_pack(
        &mut self,
        mut body_bytes: Bytes,
    ) -> Result<BytesMut> {
        if body_bytes.starts_with(&[b'P', b'A', b'C', b'K']) {
            let mut command = self.command_list.pop().unwrap();
            let temp_file = format!("./temp-{}.pack", Utc::now().timestamp());
            let mut file = OpenOptions::new()
                .write(true)
                .create(true)
                .open(&temp_file)
                .unwrap();
            file.write(&body_bytes).unwrap();
            let decoded_pack = command
                .unpack(&mut std::fs::File::open(&temp_file).unwrap())
                .unwrap();
            let save_result = self
                .storage
                .save_packfile(decoded_pack, &self.path)
                .await
                .unwrap();
            if !save_result {
                command.failed(String::from("db operation failed"));
            }

            // After receiving the pack data from the sender, the receiver sends a report
            let mut report_status = BytesMut::new();
            // TODO: replace this hard code "unpack ok\n"
            add_pkt_line_string(&mut report_status, "unpack ok\n".to_owned());
            for command in &self.command_list {
                add_pkt_line_string(&mut report_status, command.status.clone());
            }
            report_status.put(&PKT_LINE_END_MARKER[..]);

            let length = report_status.len();
            let buf = self.build_side_band_format(report_status, length);

            Ok(buf)
        } else {
            // let mut command_status: Vec<String> = vec![];
            tracing::debug!("bytes from client: {:?}", body_bytes);
            let (_, mut pkt_line) = read_pkt_line(&mut body_bytes);
            let command = self.parse_ref_update(&mut pkt_line);
            self.parse_capabilities(&String::from_utf8(pkt_line.to_vec()).unwrap());
            tracing::debug!("comamnd: {:?}, caps:{:?}", command, self.capabilities);
            self.command_list.push(command);
            Ok(BytesMut::new())
        }

        // if body_bytes.copy_to_bytes(4).to_vec() == PKT_LINE_END_MARKER {
        // }
    }

    // if capabilities contains SideBand/64k process data with sideband format else do nothing
    pub fn build_side_band_format(&self, from_bytes: BytesMut, length: usize) -> BytesMut {
        let capabilities = &self.capabilities;
        if capabilities.contains(&Capability::SideBand)
            || capabilities.contains(&Capability::SideBand64k)
        {
            let mut to_bytes = BytesMut::new();
            let length = length + 5;
            to_bytes.put(Bytes::from(format!("{length:04x}")));
            to_bytes.put_u8(SideBind::PackfileData.value());
            to_bytes.put(from_bytes);
            to_bytes.put(&PKT_LINE_END_MARKER[..]);
            return to_bytes;
        }
        from_bytes
    }
    pub fn build_smart_reply(&self, ref_list: &Vec<String>, service: String) -> BytesMut {
        let mut pkt_line_stream = BytesMut::new();
        if self.protocol == Protocol::Http {
            add_pkt_line_string(&mut pkt_line_stream, format!("# service={}\n", service));
            pkt_line_stream.put(&PKT_LINE_END_MARKER[..]);
        }

        for ref_line in ref_list {
            add_pkt_line_string(&mut pkt_line_stream, ref_line.to_string());
        }
        pkt_line_stream.put(&PKT_LINE_END_MARKER[..]);
        pkt_line_stream
    }

    pub fn parse_capabilities(&mut self, cap_str: &str) {
        let cap_vec: Vec<_> = cap_str.split(" ").collect();
        for cap in cap_vec {
            let res = cap.trim().parse::<Capability>();
            if let Ok(cap) = res {
                self.capabilities.push(cap);
            }
        }
    }

    // the first line contains the capabilities
    pub fn parse_ref_update(&self, pkt_line: &mut Bytes) -> RefCommand {
        let command = RefCommand::new(
            read_until_white_space(pkt_line),
            read_until_white_space(pkt_line),
            read_until_white_space(pkt_line),
        );
        command
    }
}

fn read_until_white_space(bytes: &mut Bytes) -> String {
    let mut buf = Vec::new();
    while let c = bytes.get_u8() {
        if c.is_ascii_whitespace() {
            break;
        }
        buf.push(c);
    }
    String::from_utf8(buf).unwrap()
}

fn find_common_base(
    mut obj_id: Hash,
    object_root: &Path,
    have: &[String],
) -> HashMap<Hash, MetaData> {
    let mut result: HashMap<Hash, MetaData> = HashMap::new();
    let mut basic_objects: HashSet<Hash> = HashSet::new();
    let common_base_commit: Commit;
    let mut commits: Vec<Commit> = vec![];
    loop {
        let commit = Commit::parse_from_file(
            object_root
                .join(obj_id.to_folder())
                .join(obj_id.to_filename()),
        );
        // stop when find common base commit
        if have.contains(&obj_id.to_plain_str()) {
            common_base_commit = commit;
            tracing::info!("found common base commit:{}", obj_id);
            break;
        }
        commits.push(commit.clone());
        result.insert(commit.meta.id, commit.meta);

        let parent_ids = commit.parent_tree_ids;

        if parent_ids.len() == 1 {
            obj_id = parent_ids[0];
        } else {
            tracing::error!("multi branch not supported yet");
            todo!();
        }
    }

    // init basic hashset by common base commit
    parse_tree(
        object_root,
        common_base_commit.tree_id,
        &mut result,
        &mut basic_objects,
        true,
    );
    for commit in commits.iter().rev() {
        let tree_id = commit.tree_id;
        parse_tree(object_root, tree_id, &mut result, &mut basic_objects, false);
    }
    result
}

fn parse_tree(
    object_root: &Path,
    tree_id: Hash,
    result: &mut HashMap<Hash, MetaData>,
    basic_objects: &mut HashSet<Hash>,
    init_basic: bool,
) {
    if basic_objects.contains(&tree_id) {
        return;
    }
    let tree = Tree::parse_from_file(
        object_root
            .join(tree_id.to_folder())
            .join(tree_id.to_filename()),
    );
    basic_objects.insert(tree_id);
    if !init_basic {
        result.insert(tree_id, tree.meta.to_owned());
    }

    for tree_item in tree.tree_items {
        // this itme has been parsed
        if basic_objects.contains(&tree_item.id) {
            continue;
        }
        match tree_item.item_type {
            TreeItemType::Blob => {
                if !init_basic {
                    let blob = Blob::parse_from_file(
                        object_root
                            .join(tree_item.id.to_folder())
                            .join(tree_item.id.to_filename()),
                    );
                    result.insert(blob.meta.id, blob.meta);
                }
            }
            TreeItemType::BlobExecutable => todo!(),
            TreeItemType::Tree => {
                parse_tree(object_root, tree_item.id, result, basic_objects, init_basic);
            }
            TreeItemType::Commit => todo!(),
            TreeItemType::Link => todo!(),
        }
        basic_objects.insert(tree_item.id);
    }
}

fn add_pkt_line_string(pkt_line_stream: &mut BytesMut, buf_str: String) {
    let buf_str_length = buf_str.len() + 4;
    pkt_line_stream.put(Bytes::from(format!("{buf_str_length:04x}")));
    pkt_line_stream.put(buf_str.as_bytes());
}

/// Read a single pkt-format line from body chunk, return the single line length and line bytes
pub fn read_pkt_line(bytes: &mut Bytes) -> (usize, Bytes) {
    let pkt_length = bytes.copy_to_bytes(4);
    let pkt_length =
        usize::from_str_radix(&String::from_utf8(pkt_length.to_vec()).unwrap(), 16).unwrap();

    if pkt_length == 0 {
        return (0, Bytes::new());
    }
    // this operation will change the original bytes
    let pkt_line = bytes.copy_to_bytes(pkt_length - 4);

    (pkt_length, pkt_line)
}

#[cfg(test)]
pub mod test {
    use bytes::{Bytes, BytesMut};

    use super::{add_pkt_line_string, read_pkt_line};

    #[test]
    pub fn test_read_pkt_line() {
        let mut bytes = Bytes::from_static(b"001e# service=git-upload-pack\n");
        let (pkt_length, pkt_line) = read_pkt_line(&mut bytes);
        assert_eq!(pkt_length, 30);
        assert_eq!(&pkt_line[..], b"# service=git-upload-pack\n");
    }

    // #[test]
    // pub fn test_build_smart_reply() {
    //     let ref_list = vec![String::from("7bdc783132575d5b3e78400ace9971970ff43a18 refs/heads/master\0report-status report-status-v2 thin-pack side-band side-band-64k ofs-delta shallow deepen-since deepen-not deepen-relative multi_ack_detailed no-done object-format=sha1\n")];
    //     let pkt_line_stream = build_smart_reply(&ref_list, String::from("git-upload-pack"));
    //     assert_eq!(&pkt_line_stream[..], b"001e# service=git-upload-pack\n000000e87bdc783132575d5b3e78400ace9971970ff43a18 refs/heads/master\0report-status report-status-v2 thin-pack side-band side-band-64k ofs-delta shallow deepen-since deepen-not deepen-relative multi_ack_detailed no-done object-format=sha1\n0000")
    // }

    #[test]
    pub fn test_add_to_pkt_line() {
        let mut buf = BytesMut::new();
        add_pkt_line_string(
            &mut buf,
            format!(
                "ACK {} common\n",
                "7bdc783132575d5b3e78400ace9971970ff43a18"
            ),
        );
        add_pkt_line_string(
            &mut buf,
            format!("ACK {} ready\n", "7bdc783132575d5b3e78400ace9971970ff43a18"),
        );
        assert_eq!(&buf.freeze()[..], b"0038ACK 7bdc783132575d5b3e78400ace9971970ff43a18 common\n0037ACK 7bdc783132575d5b3e78400ace9971970ff43a18 ready\n");
    }
}
