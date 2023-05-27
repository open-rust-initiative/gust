//!
//!
//!
use std::collections::HashMap;

use std::net::SocketAddr;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;

use anyhow::Result;
use axum::body::Body;
use axum::extract::{Query, State};
use axum::http::{Response, StatusCode};
use axum::routing::get;
use axum::{Router, Server};
use hyper::{Request, Uri};
use regex::Regex;
use serde::Deserialize;

use crate::git::lfs;
use crate::git::lfs::structs::*;
use crate::git::protocol::{http, PackProtocol, Protocol};
use crate::gust::driver::database::mysql;
use crate::gust::driver::ObjectStorage;
use crate::ServeConfig;

#[derive(Clone)]
pub struct AppState<T: ObjectStorage> {
    pub storage: T,
    pub config: ServeConfig,
}

#[derive(Deserialize, Debug)]
struct GetParams {
    pub service: Option<String>,
    pub refspec: Option<String>,
    pub id: Option<String>,
    pub path: Option<String>,
    pub limit: Option<String>,
    pub cursor: Option<String>,
}

pub fn remove_git_suffix(uri: Uri, git_suffix: &str) -> PathBuf {
    PathBuf::from(uri.path().replace(".git", "").replace(git_suffix, ""))
}

pub async fn http_server(config: &ServeConfig) -> Result<(), Box<dyn std::error::Error>> {
    let state = AppState {
        storage: mysql::init().await,
        config: config.to_owned(),
    };

    let ServeConfig {
        host,
        port,
        key_path,
        cert_path,
        lfs_content_path,
    } = config;
    let server_url = format!("{}:{}", host, port);

    let app = Router::new()
        .route(
            "/*path",
            get(get_method_router)
                .post(post_method_router)
                .put(put_method_router),
        )
        .with_state(state);

    let addr = SocketAddr::from_str(&server_url).unwrap();
    Server::bind(&addr).serve(app.into_make_service()).await?;

    Ok(())
}

/// Discovering Reference
async fn get_method_router<T>(
    state: State<AppState<T>>,
    Query(params): Query<GetParams>,
    uri: Uri,
) -> Result<Response<Body>, (StatusCode, String)>
where
    T: ObjectStorage,
{
    // Routing LFS services.
    if Regex::new(r"/objects/[a-z0-9]+$")
        .unwrap()
        .is_match(uri.path())
    {
        // Retrieve the `:oid` field from path.
        let path = uri.path().to_owned();
        let tokens: Vec<&str> = path.split('/').collect();
        // The `:oid` field is the last field.
        return lfs::http::lfs_download_object(state, tokens[tokens.len() - 1]).await;
    } else if Regex::new(r"/locks$").unwrap().is_match(uri.path()) {
        // Load query parameters into struct.
        let lock_list_query = LockListQuery {
            path: params.path,
            id: params.id,
            cursor: params.cursor,
            limit: params.limit,
            refspec: params.refspec,
        };
        return lfs::http::lfs_retrieve_lock(state, lock_list_query).await;
    }

    if !Regex::new(r"/info/refs$").unwrap().is_match(uri.path()) {
        return Err((
            StatusCode::FORBIDDEN,
            String::from("Operation not supported\n"),
        ));
    }
    let service_name = params.service.unwrap();
    if service_name == "git-upload-pack" || service_name == "git-receive-pack" {
        let mut pack_protocol = PackProtocol::new(
            remove_git_suffix(uri, "/info/refs"),
            &service_name,
            Arc::new(state.storage.clone()),
            Protocol::Http,
        );
        let mut headers = HashMap::new();
        headers.insert(
            "Content-Type".to_string(),
            format!(
                "application/x-{}-advertisement",
                pack_protocol.service_type.unwrap().to_string()
            ),
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

        let pkt_line_stream = pack_protocol.git_info_refs().await;
        let body = Body::from(pkt_line_stream.freeze());
        Ok(resp.body(body).unwrap())
    } else {
        Err((
            StatusCode::FORBIDDEN,
            String::from("Operation not supported\n"),
        ))
    }
}

async fn post_method_router<T>(
    state: State<AppState<T>>,
    uri: Uri,
    req: Request<Body>,
) -> Result<Response<Body>, (StatusCode, String)>
where
    T: ObjectStorage + 'static,
{
    // Routing LFS services.
    if Regex::new(r"/locks/verify$").unwrap().is_match(uri.path()) {
        return lfs::http::lfs_verify_lock(state, req).await;
    } else if Regex::new(r"/locks$").unwrap().is_match(uri.path()) {
        return lfs::http::lfs_create_lock(state, req).await;
    } else if Regex::new(r"/unlock$").unwrap().is_match(uri.path()) {
        // Retrieve the `:id` field from path.
        let path = uri.path().to_owned();
        let tokens: Vec<&str> = path.split('/').collect();
        // The `:id` field is just ahead of the last field.
        return lfs::http::lfs_delete_lock(state, tokens[tokens.len() - 2], req).await;
    } else if Regex::new(r"/objects/batch$").unwrap().is_match(uri.path()) {
        return lfs::http::lfs_process_batch(state, req).await;
    }

    if Regex::new(r"/git-upload-pack$")
        .unwrap()
        .is_match(uri.path())
    {
        git_upload_pack(state, remove_git_suffix(uri, "/git-upload-pack"), req).await
    } else if Regex::new(r"/git-receive-pack$")
        .unwrap()
        .is_match(uri.path())
    {
        git_receive_pack(state, remove_git_suffix(uri, "/git-receive-pack"), req).await
    } else {
        Err((
            StatusCode::FORBIDDEN,
            String::from("Operation not supported"),
        ))
    }
}

async fn put_method_router<T>(
    state: State<AppState<T>>,
    uri: Uri,
    req: Request<Body>,
) -> Result<Response<Body>, (StatusCode, String)>
where
    T: ObjectStorage + 'static,
{
    if Regex::new(r"/objects/[a-z0-9]+$")
        .unwrap()
        .is_match(uri.path())
    {
        // Retrieve the `:oid` field from path.
        let path = uri.path().to_owned();
        let tokens: Vec<&str> = path.split('/').collect();
        // The `:oid` field is the last field.
        lfs::http::lfs_upload_object(state, tokens[tokens.len() - 1], req).await
    } else {
        Err((
            StatusCode::FORBIDDEN,
            String::from("Operation not supported"),
        ))
    }
}

/// Smart Service git-upload-pack, handle git pull and clone
async fn git_upload_pack<T>(
    state: State<AppState<T>>,
    path: PathBuf,
    req: Request<Body>,
) -> Result<Response<Body>, (StatusCode, String)>
where
    T: ObjectStorage + 'static,
{
    let pack_protocol =
        PackProtocol::new(path, "", Arc::new(state.storage.clone()), Protocol::Http);

    http::git_upload_pack(req, pack_protocol).await
}

// http://localhost:8000/org1/apps/App2.git
// http://localhost:8000/org1/libs/lib1.git
/// Smart Service git-receive-pack, handle git push
async fn git_receive_pack<T>(
    state: State<AppState<T>>,
    path: PathBuf,
    req: Request<Body>,
) -> Result<Response<Body>, (StatusCode, String)>
where
    T: ObjectStorage + 'static,
{
    tracing::info!("req: {:?}", req);
    let pack_protocol =
        PackProtocol::new(path, "", Arc::new(state.storage.clone()), Protocol::Http);
    http::git_receive_pack(req, pack_protocol).await
}
