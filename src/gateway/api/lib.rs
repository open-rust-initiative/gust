//!
//!
//!
use std::collections::HashMap;

use std::env;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;

use anyhow::Result;
use axum::body::Body;
use axum::extract::{Path, Query, State};
use axum::http::{Response, StatusCode};
use axum::routing::{get, post};
use axum::{Router, Server};
use hyper::Request;
use serde::{Deserialize, Serialize};
use tower::ServiceBuilder;
use tower_cookies::CookieManagerLayer;

use crate::git::protocol::{http, PackProtocol, Protocol};
use crate::gust::driver::database::mysql;
use crate::gust::driver::ObjectStorage;

#[derive(Clone)]
struct AppState<T: ObjectStorage> {
    storage: T,
}

#[derive(Deserialize, Debug)]
struct ServiceName {
    pub service: String,
}

//TODO update this
#[derive(Debug, Deserialize, Serialize)]
pub struct Params {
    pub path: String,
    // pub path2: String,
    pub repo: String,
}

impl Params {
    pub fn get_path(&self) -> PathBuf {
        PathBuf::from(self.path.clone())
            // .join(&self.path2)
            .join(self.repo.trim_end_matches(".git"))
    }
}

pub async fn http_server() -> Result<(), Box<dyn std::error::Error>> {
    let host = env::var("HOST").expect("HOST is not set in .env file");
    let port = env::var("PORT").expect("PORT is not set in .env file");
    let server_url = format!("{}:{}", host, port);

    let state = AppState {
        storage: mysql::init().await,
    };

    let git_routes = Router::new()
        .route("/:repo/info/refs", get(git_info_refs))
        .route("/:repo/git-upload-pack", post(git_upload_pack))
        .route("/:repo/git-receive-pack", post(git_receive_pack));

    let app = Router::new()
        .nest("/:path/", git_routes)
        .layer(ServiceBuilder::new().layer(CookieManagerLayer::new()))
        .with_state(state);

    let addr = SocketAddr::from_str(&server_url).unwrap();
    Server::bind(&addr).serve(app.into_make_service()).await?;

    Ok(())
}

/// Discovering Reference
async fn git_info_refs<T>(
    state: State<AppState<T>>,
    Query(service): Query<ServiceName>,
    Path(params): Path<Params>,
) -> Result<Response<Body>, (StatusCode, String)>
where
    T: ObjectStorage,
{
    let service_name = service.service;

    if service_name == "git-upload-pack" || service_name == "git-receive-pack" {
        let mut pack_protocol = PackProtocol::new(
            params.get_path(),
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
            String::from("Operation not supported"),
        ))
    }
}

/// Smart Service git-upload-pack, handle git pull and clone
async fn git_upload_pack<T>(
    state: State<AppState<T>>,
    Path(params): Path<Params>,
    req: Request<Body>,
) -> Result<Response<Body>, (StatusCode, String)>
where
    T: ObjectStorage + 'static,
{
    let pack_protocol = PackProtocol::new(
        params.get_path(),
        "",
        Arc::new(state.storage.clone()),
        Protocol::Http,
    );

    http::git_upload_pack(req, pack_protocol).await
}

// http://localhost:8000/org1/apps/App2.git
// http://localhost:8000/org1/libs/lib1.git
/// Smart Service git-receive-pack, handle git push
async fn git_receive_pack<T>(
    state: State<AppState<T>>,
    Path(params): Path<Params>,
    req: Request<Body>,
) -> Result<Response<Body>, (StatusCode, String)>
where
    T: ObjectStorage + 'static,
{
    tracing::info!("req: {:?}", req);
    let pack_protocol = PackProtocol::new(
        params.get_path(),
        "",
        Arc::new(state.storage.clone()),
        Protocol::Http,
    );
    http::git_receive_pack(req, pack_protocol).await
}
