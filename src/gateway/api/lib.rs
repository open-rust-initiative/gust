//!
//!
//!

use anyhow::Result;
use axum::body::Body;
use axum::http::{Response, StatusCode};
use axum::routing::{get, post};
use axum::{Router, Server};
use bytes::{BufMut, Bytes, BytesMut};
use git::pack::Pack;
use hyper::body::Sender;
use log::info;

use tokio::{
    fs::File,
    io::{AsyncReadExt, BufReader},
    process::{ChildStdout, Command},
};

use std::collections::HashMap;
use std::fs::DirEntry;
use std::io::Error;
use std::path::PathBuf;
use std::{env, net::SocketAddr, process::Stdio};

use std::str::FromStr;
use tower::ServiceBuilder;
use tower_cookies::{CookieManagerLayer, Cookies};

use crate::git;

#[tokio::main]
pub(crate) async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env::set_var("RUST_LOG", "debug");
    tracing_subscriber::fmt::init();

    dotenvy::dotenv().ok();
    let db_url = env::var("DATABASE_URL").expect("DATABASE_URL is not set in .env file");
    let host = env::var("HOST").expect("HOST is not set in .env file");
    let port = env::var("PORT").expect("PORT is not set in .env file");
    let server_url = format!("{}:{}", host, port);
    let app = Router::new()
        .route("/info/refs", get(git_info_refs))
        .route("/git-upload-pack", post(git_upload_pack))
        .layer(
            ServiceBuilder::new().layer(CookieManagerLayer::new()), // .layer(Extension(data_source)),
        );

    let addr = SocketAddr::from_str(&server_url).unwrap();
    Server::bind(&addr).serve(app.into_make_service()).await?;

    Ok(())
}

async fn git_info_refs(cookies: Cookies) -> Result<Response<Body>, (StatusCode, &'static str)> {
    let mut cmd = Command::new("git");
    let work_dir = PathBuf::from("~/").join("freighter");
    // git 数据检查
    cmd.args([
        "upload-pack",
        // "--http-backend-info-refs",
        "--stateless-rpc",
        "--advertise-refs",
        work_dir.join("crates.io-index").to_str().unwrap(),
    ]);
    cmd.stdin(Stdio::piped()).stdout(Stdio::piped());

    let p = cmd.spawn().unwrap();

    let git_output = BufReader::new(p.stdout.unwrap());
    let mut headers = HashMap::new();
    headers.insert(
        "Content-Type".to_string(),
        "application/x-git-upload-pack-advertisement".to_string(),
    );
    headers.insert(
        "Cache-Control".to_string(),
        "no-cache, max-age=0, must-revalidate".to_string(),
    );
    info!("headers: {:?}", headers);
    let mut resp = Response::builder();
    for (key, val) in headers {
        resp = resp.header(&key, val);
    }

    let (sender, body) = Body::channel();
    tokio::spawn(send_refs(sender, git_output));

    let resp = resp.body(body).unwrap();
    Ok(resp)
}

async fn git_upload_pack(cookies: Cookies) -> Result<Response<Body>, (StatusCode, &'static str)> {
    let work_dir = PathBuf::from(env::var("WORK_DIR").expect("WORK_DIR is not set in .env file"));
    let object_root = work_dir.join("crates.io-index/.git/objects/pack");
    // let pack = build_pack(work_dir.clone());


    let paths = std::fs::read_dir(&object_root).unwrap();
    for path in paths {
        if let Some(pack_file) = find_pack_file(path, &object_root) {
                info!("decode file: {:?}", pack_file);
            let decoded_pack = Pack::decode_file(&pack_file);
            for (hash, meta) in &decoded_pack.result.by_hash {
                let res = meta.write_to_file("./".to_owned());
                info!("res:{:?}", res);
            }
        }
    }
    // let final_pack = Pack::pack_object_dir(loose_root_path.to_str().unwrap(), "./");
    // todo: replace with final pack
    let file = object_root.join("pack-aa2ab2eb4e6b37daf6dcadf1b6f0d8520c14dc89.pack");

    let pack_file = File::open(file).await.unwrap();

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

    info!("headers: {:?}", headers);
    let mut resp = Response::builder();
    for (key, val) in headers {
        resp = resp.header(&key, val);
    }

    let (sender, body) = Body::channel();
    tokio::spawn(send_pack(sender, reader));
    let resp = resp.body(body).unwrap();
    Ok(resp)
}

async fn send_refs(
    mut sender: Sender,
    mut git_output: BufReader<ChildStdout>,
) -> Result<(), (StatusCode, &'static str)> {
    let mut buf = BytesMut::new();
    buf.put(&b"001e# service=git-upload-pack\n0000"[..]);
    sender.send_data(buf.freeze()).await.unwrap();

    loop {
        let mut bytes_out = BytesMut::new();
        git_output.read_buf(&mut bytes_out).await.unwrap();
        if bytes_out.is_empty() {
            println!("send:empty");
            return Ok(());
        }
        println!("send: bytes_out: {:?}", bytes_out.clone().freeze());
        sender.send_data(bytes_out.freeze()).await.unwrap();
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

/// obtain the file ends with name pack and filter the dir entry
fn find_pack_file(path: Result<DirEntry, Error>, object_root: &PathBuf) -> Option<String> {
    if let Ok(pack_file) = path {
        if pack_file.file_type().unwrap().is_file() {
            let file_name = pack_file.file_name();
            let file_name = file_name.to_str().unwrap();
            if file_name.ends_with("pack") {
                let file_path = object_root.join(file_name);
                return Some(file_path.to_str().unwrap().to_owned());
            }
        }
    }
    None
}
