//!
//!
//!

use anyhow::Result;
use axum::body::Body;
use axum::extract::{BodyStream, Path, Query};
use axum::http::{Response, StatusCode};
use axum::middleware::Next;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{middleware, Router, Server};
use bytes::Bytes;

use hyper::Request;

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::{env, net::SocketAddr};

use std::str::FromStr;
use tower::ServiceBuilder;
use tower_cookies::CookieManagerLayer;

use crate::git::protocal::HttpProtocol;

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
        .route("/:repo/info/refs", get(git_info_refs))
        .route("/:repo/git-upload-pack", post(git_upload_pack))
        .route("/:repo/git-receive-pack", post(git_receive_pack))
        .layer(
            ServiceBuilder::new()
                .layer(CookieManagerLayer::new())
                .layer(middleware::from_fn(print_request_response)), // .layer(Extension(data_source)),
        );

    let addr = SocketAddr::from_str(&server_url).unwrap();
    Server::bind(&addr).serve(app.into_make_service()).await?;

    Ok(())
}

#[derive(Deserialize, Debug)]
struct ServiceName {
    pub service: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct Params {
    pub repo: String,
}

async fn git_info_refs(
    Query(service): Query<ServiceName>,
    Path(params): Path<Params>,
    body: Request<Body>,
) -> Result<Response<Body>, (StatusCode, String)> {
    // tracing::info!("service: {:?}, ", service);
    tracing::info!("{:?}", params.repo);
    let work_dir = PathBuf::from("~/").join(params.repo);

    let service_name = service.service;
    if service_name == "git-upload-pack" {
        HttpProtocol::git_info_refs(work_dir, service_name).await
    } else if service_name == "git-receive-pack" {
        HttpProtocol::git_info_refs(work_dir, service_name).await
    } else {
        return Err((StatusCode::NOT_FOUND, String::from("Operation not supported")));
    }
}

async fn git_upload_pack(
    Path(params): Path<Params>,
    mut stream: BodyStream,
) -> Result<Response<Body>, (StatusCode, String)> {
    tracing::info!("{:?}", params.repo);
    HttpProtocol::git_upload_pack(stream).await
}
async fn git_receive_pack(
    Path(params): Path<Params>,
    mut stream: BodyStream,
) -> Result<Response<Body>, (StatusCode, String)> {
    tracing::info!("{:?}", params.repo);
    let work_dir = PathBuf::from("~/").join(params.repo);

    HttpProtocol::git_receive_pack(work_dir, stream).await
}

/// ### References Codes
///
/// - [axum][https://github.com/tokio-rs/axum/blob/main/examples/print-request-response/src/main.rs].
///          
///
/// print request and response
async fn print_request_response(
    req: Request<Body>,
    next: Next<Body>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let (parts, body) = req.into_parts();
    let bytes = buffer_and_print(&format!("request: {}", parts.uri.to_string()), body).await?;
    let req = Request::from_parts(parts, Body::from(bytes));

    let res = next.run(req).await;

    let (parts, body) = res.into_parts();
    let bytes = buffer_and_print("response", body).await?;
    let res = Response::from_parts(parts, Body::from(bytes));

    Ok(res)
}

async fn buffer_and_print<B>(direction: &str, body: B) -> Result<Bytes, (StatusCode, String)>
where
    B: axum::body::HttpBody<Data = Bytes>,
    B::Error: std::fmt::Display,
{
    let bytes = match hyper::body::to_bytes(body).await {
        Ok(bytes) => bytes,
        Err(err) => {
            return Err((
                StatusCode::BAD_REQUEST,
                format!("failed to read {} body: {}", direction, err),
            ));
        }
    };

    if let Ok(body) = std::str::from_utf8(&bytes) {
        tracing::debug!("{} body = {:?}", direction, body);
    }

    Ok(bytes)
}
