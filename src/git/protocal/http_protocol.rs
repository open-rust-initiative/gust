use anyhow::Result;
use axum::body::Body;
use axum::extract::{BodyStream};
use axum::http::{Response, StatusCode};

use bytes::{Buf, BufMut, Bytes, BytesMut};
use futures::{StreamExt};
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
        let head_commit = "ffcb773734b46607d070ba4ad0559aac9496d9db";
        let mut commit_ids: Vec<(String, String)> = vec![];
        commit_ids.push((head_commit.to_string(), " HEAD\0multi_ack thin-pack side-band side-band-64k ofs-delta shallow deepen-since deepen-not deepen-relative no-progress include-tag multi_ack_detailed no-done symref=HEAD:refs/heads/master object-format=sha1 agent=git/2.38.1\n".to_string()));
        commit_ids.push((head_commit.to_string(), " refs/heads/master\n".to_string()));

        let mut buf = BytesMut::new();

        let first_pkt_line = format!("# service={}\n", service);
        let start_length = first_pkt_line.len() + 4;
        buf.put(Bytes::from(format!("{start_length:04x}")));
        buf.put(first_pkt_line.as_bytes());
        buf.put(&b"0000"[..]);

        for (commit_id, param) in commit_ids {
            let length = commit_id.len() + param.len() + 4;
            buf.put(Bytes::from(format!("{length:04x}")));
            buf.put(commit_id.as_bytes());
            buf.put(param.as_bytes());
        }
        buf.put(&b"0000"[..]);

        let body = Body::from(buf.freeze());
        let resp = resp.body(body).unwrap();
        Ok(resp)
    }

    pub async fn git_upload_pack(
        // work_dir: PathBuf,
        mut stream: BodyStream,
    ) -> Result<Response<Body>, (StatusCode, String)> {
        let mut bytes = stream.next().await.unwrap().unwrap();
        let mut want: Vec<String> = Vec::new();
        loop {
            tracing::info!("origin bytes: {:?}", bytes);
            let mut buf = bytes.take(4);
            let mut dst = vec![];
            dst.put(&mut buf);
            let bytes_take =
                usize::from_str_radix(&String::from_utf8(dst.clone()).unwrap(), 16).unwrap();
            tracing::info!("bytes want: {:?}", bytes_take);

            // skip 4 bytes
            let buf = buf.into_inner();
            if bytes_take == 0 {
                bytes = buf;
                continue;
            }
            let mut buf = buf.take(bytes_take - 4);
            dst.clear();
            dst.put(&mut buf);

            tracing::info!("read line: {:?}", String::from_utf8(dst.clone()).unwrap());
            let commands = dst[0..4].to_owned();
            if commands == b"want" {
                want.push(String::from_utf8(dst[5..45].to_vec()).unwrap());
            } else if commands == b"done" {
                break;
            }
            bytes = buf.into_inner();
        }
        tracing::info!("want commands: {:?}", want);
        let work_dir =
            PathBuf::from(env::var("WORK_DIR").expect("WORK_DIR is not set in .env file"));
        let object_root = work_dir.join("crates.io-index/.git/objects");
        // let pack = build_pack(work_dir.clone());

        let entries = fs::read_dir(&object_root)
            .unwrap()
            .map(|res| res.map(|e| e.path()))
            .collect::<Result<Vec<_>, io::Error>>()
            .unwrap();
        // entry length less than 2 represents only contains pack and info dir
        if entries.len() == 2 {
            let pack_root = object_root.join("pack");
            let decoded_pack = Pack::multi_decode(pack_root.to_str().unwrap()).unwrap();
            for (hash, meta) in &decoded_pack.result.by_hash {
                let res = meta.write_to_file(object_root.to_str().unwrap().to_owned());
                tracing::info!("res:{:?}", res);
            }
        }
        // pack target object to pack file
        // let final_pack = Pack::pack_object_dir(object_root.to_str().unwrap(), "./");
        let pack_file = File::open(format!(
            "./pack-{}.pack",
            "a1bd835a33d12c185dd6bc94f7ad174a4a8ca009"
        ))
        .await
        .unwrap();
        let reader = BufReader::new(pack_file);

        let mut headers = HashMap::new();
        headers.insert(
            "Content-Type".to_string(),
            "application/x-git-upload-pack-result".to_string(),
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

        let (sender, body) = Body::channel();
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
        let mut read_pkt_line = false;
        let file = File::create("./temp.pack").await.unwrap();
        let mut buffer = BufWriter::new(file);
        while let Some(chunk) = body.next().await {
            let mut bytes = chunk.unwrap();
            if read_pkt_line {
                let res = buffer.write(&mut bytes).await;
                tracing::info!("write to PAKC: {:?}", res);
            } else {
                let pkt_length = bytes.copy_to_bytes(4);
                let pkt_length =
                    usize::from_str_radix(&String::from_utf8(pkt_length.to_vec()).unwrap(), 16)
                        .unwrap();
                let mut pkt_line = bytes.copy_to_bytes(pkt_length - 4);

                //for example
                //ffcb773734b46607d070ba4ad0559aac9496d9db
                let from_id = pkt_line.copy_to_bytes(40);
                //0662f64166cc65d5d51152aebebd6b0bc010253c
                pkt_line.copy_to_bytes(1);
                let to_id = pkt_line.copy_to_bytes(40);
                tracing::info!("from_id: {:?}, to_id: {:?}", from_id, to_id);

                if bytes.copy_to_bytes(4).to_vec() == b"0000" {
                    let res = buffer.write(&mut bytes).await;
                    tracing::info!("write to PAKC: {:?}", res);
                }
                read_pkt_line = true;
            }
        }
        buffer.flush().await.unwrap();
        let mut resp = Response::builder();
        let resp = resp.body(Body::empty()).unwrap();
        Ok(resp)
    }
}

async fn send_pack(
    mut sender: Sender,
    mut reader: BufReader<File>,
) -> Result<(), (StatusCode, &'static str)> {
    let mut nak = BytesMut::new();
    nak.put(&b"0008NAK\n"[..]);
    sender.send_data(nak.freeze()).await.unwrap();

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