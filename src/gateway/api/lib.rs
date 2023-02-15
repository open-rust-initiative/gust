//!
//!
//!
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration;
use std::{env, net::SocketAddr};

use anyhow::Result;
use axum::body::Body;
use axum::extract::{Path, Query, State};
use axum::http::{Response, StatusCode};
use axum::routing::{get, post};
use axum::{Router, Server};
use hyper::Request;
use sea_orm::{ConnectOptions, Database};
use serde::{Deserialize, Serialize};
use tower::ServiceBuilder;
use tower_cookies::CookieManagerLayer;
use tracing::log::{self};

use crate::git::protocol::HttpProtocol;
use crate::gust::driver::StorageType;

#[tokio::main]
pub(crate) async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env::set_var("RUST_LOG", "debug");
    tracing_subscriber::fmt::init();

    dotenvy::dotenv().ok();
    let db_url = env::var("DATABASE_URL").expect("DATABASE_URL is not set in .env file");
    let host = env::var("HOST").expect("HOST is not set in .env file");
    let port = env::var("PORT").expect("PORT is not set in .env file");
    let server_url = format!("{}:{}", host, port);

    let mut opt = ConnectOptions::new(db_url.to_owned());
    // max_connections is properly for double size of the cpu core
    opt.max_connections(32)
        .min_connections(8)
        .acquire_timeout(Duration::from_secs(30))
        .connect_timeout(Duration::from_secs(20))
        .idle_timeout(Duration::from_secs(8))
        .max_lifetime(Duration::from_secs(8))
        .sqlx_logging(true)
        .sqlx_logging_level(log::LevelFilter::Error);

    let state = AppState {
        stotage_type: StorageType::Mysql(
            Database::connect(opt)
                .await
                .expect("Database connection failed"),
        ),
    };

    let git_routes = Router::new()
        .route("/:repo/info/refs", get(git_info_refs))
        .route("/:repo/git-upload-pack", post(git_upload_pack))
        .route("/:repo/git-receive-pack", post(git_receive_pack))
        .route("/:repo/decode", post(decode_packfile));

    let app = Router::new()
        .nest("/:path", git_routes)
        .layer(ServiceBuilder::new().layer(CookieManagerLayer::new()))
        .with_state(state);

    let addr = SocketAddr::from_str(&server_url).unwrap();
    Server::bind(&addr).serve(app.into_make_service()).await?;

    Ok(())
}



#[derive(Clone)]
struct AppState {
    stotage_type: StorageType,
}

#[derive(Deserialize, Debug)]
struct ServiceName {
    pub service: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct Params {
    pub repo: String,
    pub path: String,
}

/// Discovering Reference
async fn git_info_refs(
    state: State<AppState>,
    Query(service): Query<ServiceName>,
    Path(params): Path<Params>,
) -> Result<Response<Body>, (StatusCode, String)> {
    let work_dir =
        PathBuf::from(env::var("WORK_DIR").expect("WORK_DIR is not set in .env file"));
    let repo_dir = work_dir.join(params.path).join(params.repo.replace(".git", ""));

    let http_protocol = HttpProtocol::default();

    if !repo_dir.exists() {
        tokio::fs::create_dir_all(&repo_dir).await.unwrap();
    }

    let service_name = service.service;
    if service_name == "git-upload-pack" || service_name == "git-receive-pack" {
        http_protocol.git_info_refs(repo_dir, service_name, &state.stotage_type).await
    } else {
        return Err((
            StatusCode::FORBIDDEN,
            String::from("Operation not supported"),
        ));
    }
}

/// Smart Service git-upload-pack
async fn git_upload_pack(
    Path(params): Path<Params>,
    req: Request<Body>,
) -> Result<Response<Body>, (StatusCode, String)> {
    tracing::info!("{:?}", params.repo);
    let http_protocol = HttpProtocol::default();
    let mut work_dir =
        PathBuf::from(env::var("WORK_DIR").expect("WORK_DIR is not set in .env file"));
    work_dir = work_dir.join(params.repo.replace(".git", ""));
    http_protocol.git_upload_pack(work_dir, req).await
}

/// Smart Service git-receive-pack
async fn git_receive_pack(
    // Extension(ref data_source): Extension<DataSource>,
    state: State<AppState>,
    Path(params): Path<Params>,
    req: Request<Body>,
) -> Result<Response<Body>, (StatusCode, String)> {
    tracing::info!("req: {:?}", req);
    let mut work_dir =
        PathBuf::from(env::var("WORK_DIR").expect("WORK_DIR is not set in .env file"));
    work_dir = work_dir.join(params.repo.replace(".git", ""));

    let http_protocol = HttpProtocol::default();
    http_protocol
        .git_receive_pack(req, work_dir, &state.stotage_type)
        .await
}

/// try to unpack all object from pack file
async fn decode_packfile() {
    let http_protocol = HttpProtocol::default();

    http_protocol.decode_packfile().await
}
