//!
//!
//!
use std::collections::HashMap;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::{env, net::SocketAddr};

use anyhow::Result;
use axum::body::Body;
use axum::extract::{Path, Query, State};
use axum::http::{Response, StatusCode};
use axum::routing::{get, post};
use axum::{Router, Server};
use hyper::Request;
use serde::{Deserialize, Serialize};
use tokio::runtime::{Runtime, Handle};
use tower::ServiceBuilder;
use tower_cookies::CookieManagerLayer;

use crate::git::protocol::ssh::SshServer;
use crate::git::protocol::HttpProtocol;
use crate::gust::driver::database::mysql;
use crate::gust::driver::ObjectStorage;

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
        .nest("/:path/:path2", git_routes)
        .layer(ServiceBuilder::new().layer(CookieManagerLayer::new()))
        .with_state(state);

    let addr = SocketAddr::from_str(&server_url).unwrap();
    Server::bind(&addr).serve(app.into_make_service()).await?;

    Ok(())
}

/// start a ssh server
pub async fn ssh_server() -> Result<(), std::io::Error> {
    // let mut host_key = Vec::new();
    // let mut file = File::open("/path/to/host/key").unwrap();
    // file.read_to_end(&mut host_key).unwrap();
    // let host_key = PrivateKey::decode_ssh_private_key(&host_key, None).unwrap();

    let client_key = thrussh_keys::key::KeyPair::generate_ed25519().unwrap();
    let client_pubkey = Arc::new(client_key.clone_public_key());

    let mut config = thrussh::server::Config::default();
    config.connection_timeout = Some(std::time::Duration::from_secs(5));
    config.auth_rejection_time = std::time::Duration::from_secs(3);
    config
        .keys
        .push(thrussh_keys::key::KeyPair::generate_ed25519().unwrap());
    let config = Arc::new(config);
    let sh = SshServer {
        client_pubkey,
        clients: Arc::new(Mutex::new(HashMap::new())),
        id: 0,
        storage: mysql::init().await,
        handle: Handle::current(),
    };
    thrussh::server::run(config, "0.0.0.0:2222", sh).await
}

#[derive(Clone)]
struct AppState<T: ObjectStorage> {
    storage: T,
}

#[derive(Deserialize, Debug)]
struct ServiceName {
    pub service: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Params {
    pub path: String,
    pub path2: String,
    pub repo: String,
}
/// http: url => params(Struct) => path 
/// ssh: url => command(String) => path
impl Params {
    pub fn get_path(&self) -> PathBuf {
        PathBuf::from(
            self.path.clone() + "/" + &self.path2 + "/" + self.repo.trim_end_matches(".git"),
        )
    }
}

/// Discovering Reference
async fn git_info_refs<T>(
    state: State<AppState<T>>,
    Query(service): Query<ServiceName>,
    Path(params): Path<Params>,
) -> Result<Response<Body>, (StatusCode, String)> 
where T: ObjectStorage + Clone {
    let service_name = service.service;

    if service_name == "git-upload-pack" || service_name == "git-receive-pack" {
        let mut http_protocol = HttpProtocol::new(params.get_path(), &service_name, state.storage.clone());
        // http_protocol.service_type(&service_name);

        let mut headers = HashMap::new();
        headers.insert(
            "Content-Type".to_string(),
            format!("application/x-{}-advertisement", http_protocol.service_type.unwrap().to_string()),
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

        let pkt_line_stream = http_protocol.git_info_refs().await;
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
where T: ObjectStorage + Clone {
    // state.storage.set_path(params);
    let http_protocol = HttpProtocol::new(params.get_path(), "", state.storage.clone());
    http_protocol.git_upload_pack(req).await
}

// http://localhost:8000/org1/apps/App2.git
// http://localhost:8000/org1/libs/lib1.git
/// Smart Service git-receive-pack, handle git push
async fn git_receive_pack<T>(
    state: State<AppState<T>>,
    Path(params): Path<Params>,
    req: Request<Body>,
) -> Result<Response<Body>, (StatusCode, String)> 
where T: ObjectStorage + Clone {
    tracing::info!("req: {:?}", req);
    let http_protocol = HttpProtocol::new(params.get_path(), "", state.storage.clone());
    http_protocol.git_receive_pack(req).await
}
