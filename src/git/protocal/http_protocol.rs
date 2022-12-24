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
use std::env;
use std::fs::{self};
use std::io::{self};
use std::path::PathBuf;

use crate::git;

use super::HttpProtocol;

#[derive(Debug, Clone)]
pub struct RefResult {
    pub ref_name: String,
    pub from_id: String,
    pub to_id: String,
    pub result: String,
}

impl HttpProtocol {
    pub async fn git_info_refs(
        work_dir: PathBuf,
        service: String,
    ) -> Result<Response<Body>, (StatusCode, String)> {
        let work_dir = PathBuf::from("~/").join("freighter");

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

        // TODO: get HEAD commmit_id of the current working directory
        let head_commit = "b2a1b00f1662679ac82272feaf8d08638e74f0eb";
        let mut ref_vec: Vec<(String, String)> = vec![];
        ref_vec.push((head_commit.to_string(), " refs/heads/master\0report-status report-status-v2 thin-pack side-band side-band-64k ofs-delta shallow deepen-since deepen-not deepen-relative multi_ack_detailed no-done object-format=sha1 agent=git/2.38.1\n".to_string()));
        ref_vec.push((
            head_commit.to_string(),
            " refs/remotes/origin/master\n".to_string(),
        ));

        let mut buf = BytesMut::new();

        let first_pkt_line = format!("# service={}\n", service);
        add_to_pkt_line(&mut buf, first_pkt_line);
        buf.put(&b"0000"[..]);

        for (commit_id, ref_name) in ref_vec {
            let refs_line = format!("{}{}", commit_id, ref_name);
            add_to_pkt_line(&mut buf, refs_line);
        }
        buf.put(&b"0000"[..]);

        tracing::info!("git_info_refs response: {:?}", buf);
        let body = Body::from(buf.freeze());
        let resp = resp.body(body).unwrap();
        Ok(resp)
    }

    pub async fn git_upload_pack(
        // work_dir: PathBuf,
        req: Request<Body>,
    ) -> Result<Response<Body>, (StatusCode, String)> {
        let (_parts, mut body) = req.into_parts();

        let mut want: Vec<String> = Vec::new();
        let mut have: Vec<String> = Vec::new();

        let mut is_done_command = false;
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
                let commands = dst[0..4].to_owned();
                if commands == b"want" {
                    want.push(String::from_utf8(dst[5..45].to_vec()).unwrap());
                } else if commands == b"done" {
                    // The done command signals to the server that the client is ready to receive its package file data.
                    is_done_command = true;
                    break;
                } else {
                    have.push(String::from_utf8(dst[5..45].to_vec()).unwrap());
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

        let mut buf = BytesMut::new();
        let (mut sender, body) = Body::channel();
        if is_done_command {
            add_to_pkt_line(
                &mut buf,
                "ACK b2a1b00f1662679ac82272feaf8d08638e74f0eb\n".to_string(),
            );
        } else {
            // multi_ack_detailed mode, the server will differentiate the ACKs where it is signaling that
            // it is ready to send data with ACK obj-id ready lines,
            // and signals the identified common commits with ACK obj-id common lines
            for commit in &have {
                add_to_pkt_line(&mut buf, format!("ACK {} common\n", commit));
            }
            add_to_pkt_line(&mut buf, format!("ACK {} ready\n", have[have.len() - 1]));

            // TODO: determine which commit id to use in ACK line
            add_to_pkt_line(
                &mut buf,
                format!("ACK {} \n", "b2a1b00f1662679ac82272feaf8d08638e74f0eb"),
            );
        }
        tracing::info!("send buf: {:?}", buf);
        sender.send_data(buf.freeze()).await.unwrap();

        let pack_file = File::open(format!(
            "./pack-{}.pack",
            "a1bd835a33d12c185dd6bc94f7ad174a4a8ca009"
        ))
        .await
        .unwrap();
        let reader = BufReader::new(pack_file);
        tokio::spawn(send_pack(sender, reader));
        let resp = resp.body(body).unwrap();
        Ok(resp)
    }

    pub async fn git_receive_pack(
        work_dir: PathBuf,
        req: Request<Body>,
    ) -> Result<Response<Body>, (StatusCode, String)> {
        // not in memory
        let (_parts, mut body) = req.into_parts();
        let mut pkt_line_parsed = false;
        let file = File::create("./temp.pack").await.unwrap();
        let mut buffer = BufWriter::new(file);
        let mut ref_result: Vec<RefResult> = vec![];
        while let Some(chunk) = body.next().await {
            let mut bytes = chunk.unwrap();
            if pkt_line_parsed {
                let res = buffer.write(&mut bytes).await;
                tracing::info!("write to PAKC: {:?}", res);
            } else {
                tracing::info!("{:?}", bytes);
                let (_pkt_length, pkt_line) = read_pkt_line(&mut bytes);
                let pkt_vec: Vec<_> = pkt_line.to_str().unwrap().split(" ").collect();
                ref_result.push(RefResult {
                    ref_name: pkt_vec[2].to_string(),
                    from_id: pkt_vec[0].to_string(),
                    to_id: pkt_vec[1].to_string(),
                    // TODO: according to the ref handle result, if pack file parsed success return ok
                    result: "ok".to_owned(),
                });

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
        let msg = "unpack ok\n";
        add_to_pkt_line(&mut buf, msg.to_owned());
        for res in ref_result {
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

async fn send_pack(
    mut sender: Sender,
    mut reader: BufReader<File>,
) -> Result<(), (StatusCode, &'static str)> {
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
        bytes_out.put_u8(b'\x01');
        bytes_out.put(&mut temp);
        // println!("send: bytes_out: {:?}", bytes_out.clone().freeze());
        sender.send_data(bytes_out.freeze()).await.unwrap();
    }
}

fn add_to_pkt_line(buf: &mut BytesMut, buf_str: String) {
    let buf_str_length = buf_str.len() + 4;
    buf.put(Bytes::from(format!("{buf_str_length:04x}")));
    buf.put(buf_str.as_bytes());
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
