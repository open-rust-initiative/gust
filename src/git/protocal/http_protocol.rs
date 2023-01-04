use anyhow::Result;
use axum::body::Body;
use axum::http::{Response, StatusCode};

use bstr::ByteSlice;
use bytes::{Buf, BufMut, Bytes, BytesMut};
use futures::StreamExt;
use git::pack::Pack;
use hyper::body::Sender;
use hyper::Request;

use tokio::io::{AsyncWriteExt, BufWriter};
use tokio::{
    fs::File,
    io::{AsyncReadExt, BufReader},
};

use std::collections::HashMap;
use std::convert::TryInto;
use std::env;
use std::path::PathBuf;
use std::str::FromStr;

use crate::git;
use crate::git::hash::Hash;
use crate::git::idx::Idx;
use crate::git::object::base::commit::Commit;
use crate::git::object::metadata::MetaData;
use crate::git::pack::cache::PackObjectCache;

use super::HttpProtocol;

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

    const CAP_LIST: &str  = "report-status report-status-v2 thin-pack side-band side-band-64k ofs-delta shallow deepen-since deepen-not deepen-relative multi_ack_detailed no-done object-format=sha1";

    const LF: char = '\n';

    const SP: char = ' ';

    const NUL: char = '\0';

    // sideband 1 will contain packfile data,
    // sideband 2 will be used for progress information that the client will generally print to stderr and
    // sideband 3 is used for error information.
    const SIDE_BAND_BYTE_1: u8 = b'\x01';

    pub async fn git_info_refs(
        &self,
        work_dir: PathBuf,
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

        let heads = work_dir.join("heads");
        let mut ref_list = vec![];
        Self::build_ref_list(heads, &mut ref_list, String::from("refs/heads/"));
        let pkt_line_stream = Self::build_smart_reply(&ref_list, service);

        tracing::info!("git_info_refs response: {:?}", pkt_line_stream);
        let body = Body::from(pkt_line_stream.freeze());
        let resp = resp.body(body).unwrap();
        Ok(resp)
    }

    fn build_ref_list(path: PathBuf, ref_list: &mut Vec<String>, mut name: String) {
        let paths = std::fs::read_dir(&path).unwrap();
        for path in paths {
            if let Ok(ref_file) = path {
                name.push_str(ref_file.file_name().to_str().unwrap());
                let object_id = std::fs::read_to_string(ref_file.path()).unwrap();
                let object_id = object_id.strip_suffix('\n').unwrap();
                let pkt_line;
                // The stream MUST include capability declarations behind a NUL on the first ref.
                if ref_list.is_empty() {
                    pkt_line = format!(
                        "{}{}{}{}{}{}",
                        object_id,
                        HttpProtocol::SP,
                        name,
                        HttpProtocol::NUL,
                        HttpProtocol::CAP_LIST,
                        HttpProtocol::LF
                    );
                } else {
                    pkt_line = format!(
                        "{}{}{}{}{}",
                        object_id,
                        HttpProtocol::SP,
                        name,
                        HttpProtocol::NUL,
                        HttpProtocol::LF
                    );
                }
                ref_list.push(pkt_line);
            }
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

    pub async fn git_upload_pack(
        &self,
        req: Request<Body>,
    ) -> Result<Response<Body>, (StatusCode, String)> {
        let (_parts, mut body) = req.into_parts();

        let mut want: Vec<String> = Vec::new();
        let mut have: Vec<String> = Vec::new();

        while let Some(chunk) = body.next().await {
            tracing::info!("chunk:{:?}", chunk);
            let mut bytes = chunk.unwrap();
            loop {
                let (bytes_take, pkt_line) = read_pkt_line(&mut bytes);
                // if read 0000
                if bytes_take == 0 {
                    if bytes.len() == 0 {
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
        let work_dir =
            PathBuf::from(env::var("WORK_DIR").expect("WORK_DIR is not set in .env file"));
        let object_root = work_dir.join("crates.io-index/.git/objects");
        // let pack = build_pack(work_dir.clone());

        // let entries = fs::read_dir(&object_root)
        //     .unwrap()
        //     .map(|res| res.map(|e| e.path()))
        //     .collect::<Result<Vec<_>, io::Error>>()
        //     .unwrap();
        // // entry length less than 2 represents only contains pack and info dir
        // if entries.len() == 2 {
        //     let pack_root = object_root.join("pack");
        //     let decoded_pack = Pack::multi_decode(pack_root.to_str().unwrap()).unwrap();
        //     for (hash, meta) in &decoded_pack.result.by_hash {
        //         let res = meta.write_to_file(object_root.to_str().unwrap().to_owned());
        //         tracing::info!("res:{:?}", res);
        //     }
        // }
        let idx_path = object_root.join("pack/pack-db444c5a50d3ff97f514825f419bc8b02f18fc7f.idx");
        let mut meta_vec: Vec<MetaData> = vec![];
        let mut decoded_pack = Pack::default();
        let mut idx = Idx::default();
        if have.is_empty() {
            // TODO git pull command
        } else {
            let pack_path =
                object_root.join("pack/pack-db444c5a50d3ff97f514825f419bc8b02f18fc7f.pack");
            let mut origin_pack_file = std::fs::File::open(pack_path).unwrap();
            // decoded_pack =
            //     Pack::decodev2(&mut origin_pack_file, Hash::from_str(&have[0]).unwrap()).unwrap();

            idx.decode_from_path(idx_path);
            meta_vec = find_common_base(
                &idx,
                Hash::from_str(&want[0]).unwrap(),
                &mut origin_pack_file,
                &have,
            );

            // let tree_id = commit.tree_id;
            // let offset = get_object_offset(&idx, tree_id).offset;
            // let meta =
            //     Pack::next_object(&mut pack_file, offset.try_into().unwrap(), &mut cache).unwrap();
            // let meta = Metadata::new(meta.t, &meta.data);
            // let tree = Tree::new(meta);
            // println!("{:?}, {:?}", tree.tree_items, tree.tree_name);
        }

        // TODO: pack target object to pack file
        // let final_pack = Pack::pack_object_dir(object_root.to_str().unwrap(), "./");
        let mut headers = HashMap::new();
        headers.insert(
            "Content-Type".to_string(),
            "application/x-git-upload-pack-result".to_string(),
        );
        headers.insert(
            "Cache-Control".to_string(),
            "no-cache, max-age=0, must-revalidate".to_string(),
        );
        let mut resp = Response::builder();
        for (key, val) in headers {
            resp = resp.header(&key, val);
        }

        let mode = &self.mode;
        let str = HttpProtocol::value_in_ack_mode(mode);

        let mut buf = BytesMut::new();
        let (mut sender, body) = Body::channel();

        // multi_ack_detailed mode, the server will differentiate the ACKs where it is signaling that
        // it is ready to send data with ACK obj-id ready lines,
        // and signals the identified common commits with ACK obj-id common lines
        for commit in &have {
            add_to_pkt_line(&mut buf, format!("ACK {} common\n", commit));
        }
        // If multi_ack_detailed and no-done are both present, then the sender is free to immediately send a pack
        // following its first "ACK obj-id ready" message.
        add_to_pkt_line(&mut buf, format!("ACK {} ready\n", have[have.len() - 1]));

        // TODO: determine which commit id to use in ACK line
        add_to_pkt_line(
            &mut buf,
            format!("ACK {} \n", "b2a1b00f1662679ac82272feaf8d08638e74f0eb"),
        );

        tracing::info!("send buf: {:?}", buf);
        sender.send_data(buf.freeze()).await.unwrap();

        let result: Vec<u8> = decoded_pack.encode(Some(meta_vec));
        // let reader = BufReader::new(result);
        tokio::spawn(send_pack(sender, result));
        let resp = resp.body(body).unwrap();
        Ok(resp)
    }

    pub async fn git_receive_pack(
        &self,
        work_dir: PathBuf,
        req: Request<Body>,
    ) -> Result<Response<Body>, (StatusCode, String)> {
        // not in memory
        let (_parts, mut body) = req.into_parts();
        let mut pkt_line_parsed = false;
        let file = File::create("./temp.pack").await.unwrap();
        let mut buffer = BufWriter::new(file);
        let mut ref_results: Vec<RefResult> = vec![];
        while let Some(chunk) = body.next().await {
            let mut bytes = chunk.unwrap();
            if pkt_line_parsed {
                let res = buffer.write(&mut bytes).await;
                tracing::info!("write to PAKC: {:?}", res);
            } else {
                tracing::info!("{:?}", bytes);
                let (_pkt_length, pkt_line) = read_pkt_line(&mut bytes);
                let pkt_vec: Vec<_> = pkt_line.to_str().unwrap().split(" ").collect();

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
                    let res = buffer.write(&mut bytes).await;
                    tracing::info!("write to PAKC: {:?}", res);
                }
                pkt_line_parsed = true;
            }
        }
        buffer.flush().await.unwrap();

        let mut headers = HashMap::new();
        headers.insert(
            "Content-Type".to_string(),
            "application/x-git-receive-pack-result".to_string(),
        );
        headers.insert(
            "Cache-Control".to_string(),
            "no-cache, max-age=0, must-revalidate".to_string(),
        );
        let mut resp = Response::builder();

        for (key, val) in headers {
            resp = resp.header(&key, val);
        }

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
}

fn find_common_base(
    idx: &Idx,
    mut obj_id: Hash,
    pack_file: &mut std::fs::File,
    have: &Vec<String>,
) -> Vec<MetaData> {
    let mut cache = PackObjectCache::default();
    let mut result: Vec<MetaData> = vec![];
    loop {
        let offset = idx.get_offset(obj_id).offset;

        let meta = Pack::next_object(pack_file, offset.try_into().unwrap(), &mut cache).unwrap();
        let meta = MetaData::new(meta.t, &meta.data);

        let commit = Commit::new(meta);
        let parent_ids = commit.parent_tree_ids;

        if parent_ids.len() == 1 {
            obj_id = parent_ids[0];
            if have.contains(&obj_id.to_plain_str()) {
                break;
            }
            result.push(commit.meta);
        } else {
            break;
        }
    }
    result
}

async fn send_pack(
    mut sender: Sender,
    // mut reader: BufReader<File>,
    result: Vec<u8>,
) -> Result<(), (StatusCode, &'static str)> {
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
        println!("send: bytes_out: {:?}", bytes_out.clone().freeze());
        sender.send_data(bytes_out.freeze()).await.unwrap();
    }
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
