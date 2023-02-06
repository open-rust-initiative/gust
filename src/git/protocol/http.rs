//!
//!
//!
//!
use std::collections::{HashMap, HashSet};
use std::env;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use anyhow::Result;
use axum::body::Body;
use axum::http::response::Builder;
use axum::http::{Response, StatusCode};
use bstr::ByteSlice;
use bytes::{Buf, BufMut, Bytes, BytesMut};
use futures::StreamExt;
use hyper::body::Sender;
use hyper::Request;
use tokio::io::{AsyncWriteExt, BufWriter};
use tokio::{
    fs::File,
    io::{AsyncReadExt, BufReader},
};

use crate::gateway::api::lib::StorageType;
use crate::git::hash::Hash;
use crate::git::object::base::blob::Blob;
use crate::git::object::base::commit::Commit;
use crate::git::object::base::tree::{Tree, TreeItemType};
use crate::git::object::metadata::MetaData;
use crate::git::pack::Pack;
use crate::git::protocol::HttpProtocol;
use crate::gust::driver::database::mysql::mysql::Mysql;
use crate::gust::driver::{BasicObject, ObjectStorage};

#[derive(Debug, Clone)]
pub struct RefResult {
    pub ref_name: String,
    pub from_id: String,
    pub to_id: String,
    pub result: String,
}

impl RefResult {
    // TODO: according to the ref handle result, returns ok if pack file parsed success
    pub fn get_result(&mut self) {
        self.result = "ok".to_owned();
    }
}

impl HttpProtocol {
    const PKT_LINE_END_MARKER: &[u8; 4] = b"0000";

    const CAP_LIST: &str = "report-status report-status-v2 thin-pack side-band side-band-64k ofs-delta shallow deepen-since deepen-not deepen-relative multi_ack_detailed no-done object-format=sha1";

    const LF: char = '\n';

    const SP: char = ' ';

    const NUL: char = '\0';

    // sideband 1 will contain packfile data,
    // sideband 2 will be used for progress information that the client will generally print to stderr and
    // sideband 3 is used for error information.
    const SIDE_BAND_BYTE_1: u8 = b'\x01';

    pub async fn git_info_refs(
        &self,
        repo_dir: PathBuf,
        service: String,
    ) -> Result<Response<Body>, (StatusCode, String)> {
        let mut headers = HashMap::new();
        headers.insert(
            "Content-Type".to_string(),
            format!("application/x-{}-advertisement", service),
        );
        headers.insert(
            "Cache-Control".to_string(),
            "no-cache, max-age=0, must-revalidate".to_string(),
        );
        tracing::info!("headers: {:?}", headers);
        let mut resp = Response::builder();
        for (key, val) in headers {
            resp = resp.header(&key, val);
        }
        let repo_git_dir = repo_dir.join(".git");
        let mut ref_list = add_head_to_ref_list(&repo_git_dir).unwrap();
        add_to_ref_list(&repo_git_dir, &mut ref_list, String::from("refs/heads/"));

        let pkt_line_stream = build_smart_reply(&ref_list, service);

        tracing::info!("git_info_refs response: {:?}", pkt_line_stream);
        let body = Body::from(pkt_line_stream.freeze());
        let resp = resp.body(body).unwrap();
        Ok(resp)
    }

    pub async fn git_upload_pack(
        &self,
        work_dir: PathBuf,
        req: Request<Body>,
    ) -> Result<Response<Body>, (StatusCode, String)> {
        let (_parts, mut body) = req.into_parts();

        let mut want: Vec<String> = Vec::new();
        let mut have: Vec<String> = Vec::new();

        while let Some(chunk) = body.next().await {
            tracing::info!("client sends :{:?}", chunk);
            let mut bytes = chunk.unwrap();
            loop {
                let (bytes_take, pkt_line) = read_pkt_line(&mut bytes);
                // if read 0000
                if bytes_take == 0 {
                    if bytes.is_empty() {
                        break;
                    }
                    continue;
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
            }
        }

        tracing::info!("want commands: {:?}, have commans: {:?}", want, have);

        let object_root = work_dir.join(".git/objects");

        let send_pack_data;
        let mut buf = BytesMut::new();

        if have.is_empty() {
            let loose_vec = Pack::find_all_loose(object_root.to_str().unwrap());
            let (mut _loose_pack, loose_data) =
                Pack::pack_loose(loose_vec, object_root.to_str().unwrap());
            send_pack_data = loose_data;
            add_to_pkt_line(&mut buf, String::from("NAK\n"));
        } else {
            let mut decoded_pack = Pack::default();
            let mut _meta_map: HashMap<Hash, MetaData> = HashMap::new();
            _meta_map = find_common_base(Hash::from_str(&want[0]).unwrap(), object_root, &have);
            send_pack_data = decoded_pack.encode(Some(_meta_map.into_values().collect()));

            // multi_ack_detailed mode, the server will differentiate the ACKs where it is signaling that
            // it is ready to send data with ACK obj-id ready lines,
            // and signals the identified common commits with ACK obj-id common lines
            for commit in &have {
                add_to_pkt_line(&mut buf, format!("ACK {} common\n", commit));
            }
            // If multi_ack_detailed and no-done are both present, then the sender is free to immediately send a pack
            // following its first "ACK obj-id ready" message.
            add_to_pkt_line(&mut buf, format!("ACK {} ready\n", have[have.len() - 1]));

            add_to_pkt_line(&mut buf, format!("ACK {} \n", have[have.len() - 1]));
        }

        let resp = build_res_header("application/x-git-upload-pack-result".to_owned());

        tracing::info!("send buf: {:?}", buf);

        let (mut sender, body) = Body::channel();
        sender.send_data(buf.freeze()).await.unwrap();

        tokio::spawn(send_pack(sender, send_pack_data));
        let resp = resp.body(body).unwrap();
        Ok(resp)
    }

    pub async fn git_receive_pack(
        &self,
        req: Request<Body>,
        _work_dir: PathBuf,
        storage: &StorageType,
    ) -> Result<Response<Body>, (StatusCode, String)> {
        // this part need to be reused？
        match storage {
            StorageType::Mysql(_) => {
                let query = Mysql::default();
                let res = query
                    .save_objects(storage, vec![BasicObject::default()])
                    .await
                    .unwrap();
                println!("{}", res);
            }
            StorageType::Filesystem => todo!(),
        };

        // not in memory
        let (_parts, mut body) = req.into_parts();
        let mut pkt_line_parsed = false;
        let file = File::create("./temp.pack").await.unwrap();
        let mut buffer = BufWriter::new(file);
        let mut ref_results: Vec<RefResult> = vec![];
        while let Some(chunk) = body.next().await {
            let mut bytes = chunk.unwrap();
            if pkt_line_parsed {
                let res = buffer.write(&bytes).await;
                tracing::info!("write to PAKC: {:?}", res);
            } else {
                tracing::info!("{:?}", bytes);
                let (_pkt_length, pkt_line) = read_pkt_line(&mut bytes);
                let pkt_vec: Vec<_> = pkt_line.to_str().unwrap().split(' ').collect();

                let mut ref_result = RefResult {
                    ref_name: pkt_vec[2].to_string(),
                    from_id: pkt_vec[0].to_string(),
                    to_id: pkt_vec[1].to_string(),
                    result: "ng".to_owned(),
                };
                ref_result.get_result();
                ref_results.push(ref_result);

                tracing::info!("pkt_line: {:?}", pkt_vec);
                //TODO: don't know what to do with multiple refs
                if bytes.copy_to_bytes(4).to_vec() == b"0000" {
                    let res = buffer.write(&bytes).await;
                    tracing::info!("write to PAKC: {:?}", res);
                }
                pkt_line_parsed = true;
            }
        }
        buffer.flush().await.unwrap();

        let resp = build_res_header("application/x-git-receive-pack-result".to_owned());

        let mut buf = BytesMut::new();
        add_to_pkt_line(&mut buf, "unpack ok\n".to_owned());
        for res in ref_results {
            let ref_res = format!("{} {}", res.result, res.ref_name);
            add_to_pkt_line(&mut buf, ref_res);
        }
        buf.put(&b"0000"[..]);

        let body = Body::from(buf.freeze());
        tracing::info!("receive pack response {:?}", body);
        let resp = resp.body(body).unwrap();
        Ok(resp)
    }

    pub async fn decode_packfile(&self) {
        let work_dir =
            PathBuf::from(env::var("WORK_DIR").expect("WORK_DIR is not set in .env file"));
        let object_root = work_dir.join("crates.io-index/.git/objects");
        let pack_path = object_root.join("pack/pack-db444c5a50d3ff97f514825f419bc8b02f18fc7f.pack");
        let mut origin_pack_file = std::fs::File::open(pack_path).unwrap();

        let decoded_pack = Pack::decode(&mut origin_pack_file).unwrap();
        for meta in decoded_pack.result.by_hash.values() {
            let res = meta.write_to_file(object_root.to_str().unwrap().to_owned());
            tracing::info!("res:{:?}", res);
        }
    }
}

fn find_common_base(
    mut obj_id: Hash,
    object_root: PathBuf,
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
        &object_root,
        common_base_commit.tree_id,
        &mut result,
        &mut basic_objects,
        true,
    );
    for commit in commits.iter().rev() {
        let tree_id = commit.tree_id;
        parse_tree(
            &object_root,
            tree_id,
            &mut result,
            &mut basic_objects,
            false,
        );
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

async fn send_pack(mut sender: Sender, result: Vec<u8>) -> Result<(), (StatusCode, &'static str)> {
    let mut reader = BufReader::new(result.as_slice());
    loop {
        let mut bytes_out = BytesMut::new();
        let mut temp = BytesMut::new();
        let length = reader.read_buf(&mut temp).await.unwrap() + 5;
        if temp.is_empty() {
            bytes_out.put_slice(b"0000");
            sender.send_data(bytes_out.freeze()).await.unwrap();
            return Ok(());
        }
        bytes_out.put(Bytes::from(format!("{length:04x}")));
        bytes_out.put_u8(HttpProtocol::SIDE_BAND_BYTE_1);
        bytes_out.put(&mut temp);
        // println!("send: bytes_out: {:?}", bytes_out.clone().freeze());
        sender.send_data(bytes_out.freeze()).await.unwrap();
    }
}

fn add_head_to_ref_list(work_dir: &PathBuf) -> Result<Vec<String>, anyhow::Error> {
    let content = std::fs::read_to_string(work_dir.join("HEAD")).unwrap();
    let content = content.replace("ref: ", "");
    let content = content.strip_suffix('\n').unwrap();
    tracing::debug!("{:?}", content);
    //use zero_id when ref_list is empty
    let zero_id = String::from_utf8_lossy(&vec![b'0'; 40]).to_string();
    let object_id = match std::fs::read_to_string(work_dir.join(content)) {
        Ok(object_id) => object_id,
        Err(_) => {
            let mut object_id = zero_id.clone();
            object_id.push('\n');
            object_id
        }
    };
    let object_id = object_id.strip_suffix('\n').unwrap();
    // The stream MUST include capability declarations behind a NUL on the first ref.
    let name = if object_id == zero_id {
        "capabilities^{}"
    } else {
        "HEAD"
    };
    let pkt_line = format!(
        "{}{}{}{}{}{}",
        object_id,
        HttpProtocol::SP,
        name,
        HttpProtocol::NUL,
        HttpProtocol::CAP_LIST,
        HttpProtocol::LF
    );
    let ref_list = vec![pkt_line];
    Ok(ref_list)
}

fn add_to_ref_list(work_dir: &PathBuf, ref_list: &mut Vec<String>, mut name: String) {
    //TOOD: need to read from .git/packed-refs after run git gc, check how git show-ref command work
    let path = work_dir.join(&name);
    let paths = std::fs::read_dir(&path).unwrap();
    for ref_file in paths.flatten() {
        name.push_str(ref_file.file_name().to_str().unwrap());
        let object_id = std::fs::read_to_string(ref_file.path()).unwrap();
        let object_id = object_id.strip_suffix('\n').unwrap();
        let pkt_line = format!(
            "{}{}{}{}",
            object_id,
            HttpProtocol::SP,
            name,
            // HttpProtocol::NUL,
            HttpProtocol::LF
        );
        ref_list.push(pkt_line);
    }
}

fn build_smart_reply(ref_list: &Vec<String>, service: String) -> BytesMut {
    let mut pkt_line_stream = BytesMut::new();
    add_to_pkt_line(&mut pkt_line_stream, format!("# service={}\n", service));
    pkt_line_stream.put(&HttpProtocol::PKT_LINE_END_MARKER[..]);

    for ref_line in ref_list {
        add_to_pkt_line(&mut pkt_line_stream, ref_line.to_string());
    }
    pkt_line_stream.put(&HttpProtocol::PKT_LINE_END_MARKER[..]);
    pkt_line_stream
}

fn add_to_pkt_line(pkt_line_stream: &mut BytesMut, buf_str: String) {
    let buf_str_length = buf_str.len() + 4;
    pkt_line_stream.put(Bytes::from(format!("{buf_str_length:04x}")));
    pkt_line_stream.put(buf_str.as_bytes());
}

/// Read a single pkt-format line from body chunk, return the single line length and line bytes
fn read_pkt_line(bytes: &mut Bytes) -> (usize, Bytes) {
    let pkt_length = bytes.copy_to_bytes(4);
    let pkt_length =
        usize::from_str_radix(&String::from_utf8(pkt_length.to_vec()).unwrap(), 16).unwrap();

    if pkt_length == 0 {
        return (0, Bytes::new());
    }
    let pkt_line = bytes.copy_to_bytes(pkt_length - 4);

    (pkt_length, pkt_line)
}

fn build_res_header(content_type: String) -> Builder {
    let mut headers = HashMap::new();
    headers.insert("Content-Type".to_string(), content_type);
    headers.insert(
        "Cache-Control".to_string(),
        "no-cache, max-age=0, must-revalidate".to_string(),
    );
    let mut resp = Response::builder();

    for (key, val) in headers {
        resp = resp.header(&key, val);
    }
    resp
}

#[cfg(test)]
pub mod test {
    use bytes::{Bytes, BytesMut};

    use super::{add_to_pkt_line, build_smart_reply, read_pkt_line};

    #[test]
    pub fn test_read_pkt_line() {
        let mut bytes = Bytes::from_static(b"001e# service=git-upload-pack\n");
        let (pkt_length, pkt_line) = read_pkt_line(&mut bytes);
        assert_eq!(pkt_length, 30);
        assert_eq!(&pkt_line[..], b"# service=git-upload-pack\n");
    }

    #[test]
    pub fn test_build_smart_reply() {
        let ref_list = vec![String::from("7bdc783132575d5b3e78400ace9971970ff43a18 refs/heads/master\0report-status report-status-v2 thin-pack side-band side-band-64k ofs-delta shallow deepen-since deepen-not deepen-relative multi_ack_detailed no-done object-format=sha1\n")];
        let pkt_line_stream = build_smart_reply(&ref_list, String::from("git-upload-pack"));
        assert_eq!(&pkt_line_stream[..], b"001e# service=git-upload-pack\n000000e87bdc783132575d5b3e78400ace9971970ff43a18 refs/heads/master\0report-status report-status-v2 thin-pack side-band side-band-64k ofs-delta shallow deepen-since deepen-not deepen-relative multi_ack_detailed no-done object-format=sha1\n0000")
    }

    #[test]
    pub fn test_add_to_pkt_line() {
        let mut buf = BytesMut::new();
        add_to_pkt_line(
            &mut buf,
            format!(
                "ACK {} common\n",
                "7bdc783132575d5b3e78400ace9971970ff43a18"
            ),
        );
        add_to_pkt_line(
            &mut buf,
            format!("ACK {} ready\n", "7bdc783132575d5b3e78400ace9971970ff43a18"),
        );
        assert_eq!(&buf.freeze()[..], b"0038ACK 7bdc783132575d5b3e78400ace9971970ff43a18 common\n0037ACK 7bdc783132575d5b3e78400ace9971970ff43a18 ready\n");
    }
}
