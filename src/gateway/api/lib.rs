//!
//!
//!
use std::collections::HashMap;

use regex::Regex;
use sea_orm::sea_query::token;

use std::env;
use std::io::prelude::*;
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
use bytes::{BufMut, Bytes, BytesMut};
use chrono::{prelude::*, Duration};
use futures::{lock, StreamExt};
use hyper::{server, Request, Uri};
use rand::prelude::*;
use serde::Deserialize;
use serde_qs;

use crate::git::lfs::structs::*;
use crate::git::protocol::{http, PackProtocol, Protocol};
use crate::gust::driver::database::mysql;
use crate::gust::driver::lfs_content_store::ContentStore;
use crate::gust::driver::ObjectStorage;
use crate::ServeConfig;

#[derive(Clone)]
struct AppState<T: ObjectStorage> {
    storage: T,
    config: ServeConfig,
}

#[derive(Deserialize, Debug)]
struct ServiceName {
    pub service: String,
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
    uri: Uri,
) -> Result<Response<Body>, (StatusCode, String)>
where
    T: ObjectStorage,
{
    // Retrieve service query field.
    let query = uri.clone();
    let service_name = query.query().unwrap_or("");

    // Routing LFS services.
    if Regex::new(r"/objects/[a-z0-9]+$")
        .unwrap()
        .is_match(uri.path())
    {
        // Retrieve the `:oid` field from path.
        let path = uri.path().to_owned();
        let tokens: Vec<&str> = path.split('/').collect();
        // The `:oid` field is the last field.
        return lfs_download_object(state, tokens[tokens.len() - 1]).await;
    } else if Regex::new(r"/locks$").unwrap().is_match(uri.path()) {
        // Load query parameters into struct.
        let lock_list_query: LockListQuery = serde_qs::from_str(service_name).unwrap();
        return lfs_retrieve_lock(state, lock_list_query).await;
    }

    if !Regex::new(r"/info/refs$").unwrap().is_match(uri.path()) {
        return Err((
            StatusCode::FORBIDDEN,
            String::from("Operation not supported\n"),
        ));
    }

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
        return lfs_verify_lock(state, req).await;
    } else if Regex::new(r"/locks$").unwrap().is_match(uri.path()) {
        return lfs_create_lock(state, req).await;
    } else if Regex::new(r"/unlock$").unwrap().is_match(uri.path()) {
        // Retrieve the `:id` field from path.
        let path = uri.path().to_owned();
        let tokens: Vec<&str> = path.split('/').collect();
        // The `:id` field is just ahead of the last field.
        return lfs_delete_lock(state, tokens[tokens.len() - 2], req).await;
    } else if Regex::new(r"/objects/batch$").unwrap().is_match(uri.path()) {
        return lfs_process_batch(state, req).await;
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
        lfs_upload_object(state, tokens[tokens.len() - 1], req).await
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

async fn lfs_retrieve_lock<T>(
    state: State<AppState<T>>,
    lock_list_query: LockListQuery,
) -> Result<Response<Body>, (StatusCode, String)>
where
    T: ObjectStorage,
{
    tracing::info!("retrieving locks: {:?}", lock_list_query);
    let repo = lock_list_query
        .refspec
        .as_ref()
        .unwrap_or(&"".to_string())
        .to_string();
    let path = match lock_list_query.path.as_ref() {
        Some(val) => val.to_owned(),
        None => "".to_owned(),
    };
    let cursor = match lock_list_query.path.as_ref() {
        Some(val) => val.to_owned(),
        None => "".to_owned(),
    };
    let limit = match lock_list_query.path.as_ref() {
        Some(val) => val.to_owned(),
        None => "".to_owned(),
    };
    let mut resp = Response::builder();
    resp = resp.header("Content-Type", "application/vnd.git-lfs+json");

    let db = Arc::new(state.storage.clone());
    let (locks, next_cursor, ok) = match db
        .lfs_get_filtered_locks(&repo, &path, &cursor, &limit)
        .await
    {
        Ok((locks, next)) => (locks, next, true),
        Err(_) => (vec![], "".to_string(), false),
    };

    let mut lock_list = LockList {
        locks: vec![],
        next_cursor: "".to_string(),
    };

    if !ok {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            "Lookup operation failed!".to_string(),
        ));
    } else {
        lock_list.locks = locks.clone();
        lock_list.next_cursor = next_cursor;
    }

    let locks_response = serde_json::to_string(&lock_list).unwrap();
    println!("{:?}", locks_response);
    let body = Body::from(locks_response);

    Ok(resp.body(body).unwrap())
}

async fn lfs_verify_lock<T>(
    state: State<AppState<T>>,
    req: Request<Body>,
) -> Result<Response<Body>, (StatusCode, String)>
where
    T: ObjectStorage,
{
    tracing::info!("req: {:?}", req);
    let mut resp = Response::builder();
    resp = resp.header("Content-Type", "application/vnd.git-lfs+json");

    let (_parts, mut body) = req.into_parts();

    let mut request_body = BytesMut::new();

    while let Some(chunk) = body.next().await {
        tracing::info!("client sends :{:?}", chunk);
        let bytes = chunk.unwrap();
        request_body.extend_from_slice(&bytes);
    }

    let verifiable_lock_request: VerifiableLockRequest =
        serde_json::from_slice(request_body.freeze().as_ref()).unwrap();
    let mut limit = verifiable_lock_request.limit.unwrap_or(0);
    if limit == 0 {
        limit = 100;
    }

    let db = Arc::new(state.storage.clone());
    let res = db
        .lfs_get_filtered_locks(
            &verifiable_lock_request.refs.name,
            &"".to_string(),
            &verifiable_lock_request
                .cursor
                .unwrap_or("".to_string())
                .to_string(),
            &limit.to_string(),
        )
        .await;

    let (locks, next_cursor, ok) = match res {
        Ok((locks, next)) => (locks, next, true),
        Err(_) => (vec![], "".to_string(), false),
    };

    let mut lock_list = VerifiableLockList {
        ours: vec![],
        theirs: vec![],
        next_cursor: "".to_string(),
    };
    tracing::info!("acquired: {:?}", lock_list);

    if !ok {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            "Lookup operation failed!".to_string(),
        ));
    } else {
        lock_list.next_cursor = next_cursor;

        for lock in locks.iter() {
            if lock.owner == None {
                lock_list.ours.push(lock.clone());
            } else {
                lock_list.theirs.push(lock.clone());
            }
        }
    }
    let locks_response = serde_json::to_string(&lock_list).unwrap();
    tracing::info!("sending: {:?}", locks_response);
    let body = Body::from(locks_response);

    Ok(resp.body(body).unwrap())
}

async fn lfs_create_lock<T>(
    state: State<AppState<T>>,
    req: Request<Body>,
) -> Result<Response<Body>, (StatusCode, String)>
where
    T: ObjectStorage,
{
    tracing::info!("req: {:?}", req);
    let mut resp = Response::builder();
    resp = resp.header("Content-Type", "application/vnd.git-lfs+json");

    let (_parts, mut body) = req.into_parts();

    let mut request_body = BytesMut::new();

    while let Some(chunk) = body.next().await {
        tracing::info!("client sends :{:?}", chunk);
        let bytes = chunk.unwrap();
        request_body.extend_from_slice(&bytes);
    }

    let lock_request: LockRequest = serde_json::from_slice(request_body.freeze().as_ref()).unwrap();
    println!("{:?}", lock_request);
    tracing::info!("acquired: {:?}", lock_request);
    let db = Arc::new(state.storage.clone());
    let res = db
        .lfs_get_filtered_locks(
            &lock_request.refs.name,
            &lock_request.path.to_string(),
            "",
            "1",
        )
        .await;

    let (locks, _, ok) = match res {
        Ok((locks, next)) => (locks, next, true),
        Err(_) => (vec![], "".to_string(), false),
    };

    if !ok {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed when filtering locks!".to_string(),
        ));
    }

    if locks.len() > 0 {
        return Err((StatusCode::CONFLICT, "Lock already exist".to_string()));
    }

    let lock = Lock {
        id: {
            let mut random_num = String::new();
            let mut rng = rand::thread_rng();
            for _ in 0..8 {
                random_num += &(rng.gen_range(0..9)).to_string();
            }
            random_num
        },
        path: lock_request.path.to_owned(),
        owner: None,
        locked_at: {
            let locked_at: DateTime<Utc> = Utc::now();
            locked_at.to_rfc3339().to_string()
        },
    };

    let ok = db
        .lfs_add_lock(&lock_request.refs.name, vec![lock.clone()])
        .await
        .is_ok();
    if !ok {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed when adding locks!".to_string(),
        ));
    }

    resp = resp.status(StatusCode::CREATED);

    let lock_response = LockResponse {
        lock,
        message: "".to_string(),
    };
    let lock_response = serde_json::to_string(&lock_response).unwrap();
    let body = Body::from(lock_response);

    Ok(resp.body(body).unwrap())
}

async fn lfs_delete_lock<T>(
    state: State<AppState<T>>,
    id: &str,
    req: Request<Body>,
) -> Result<Response<Body>, (StatusCode, String)>
where
    T: ObjectStorage,
{
    // Retrieve information from request body.
    tracing::info!("req: {:?}", req);
    let mut resp = Response::builder();
    resp = resp.header("Content-Type", "application/vnd.git-lfs+json");

    let (_parts, mut body) = req.into_parts();

    let mut request_body = BytesMut::new();

    while let Some(chunk) = body.next().await {
        tracing::info!("client sends :{:?}", chunk);
        let bytes = chunk.unwrap();
        request_body.extend_from_slice(&bytes);
    }

    if id.len() == 0 {
        return Err((StatusCode::BAD_REQUEST, "Invalid lock id!".to_string()));
    }

    if request_body.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "Deserialize operation failed!".to_string(),
        ));
    }
    let unlock_request: UnlockRequest =
        serde_json::from_slice(request_body.freeze().as_ref()).unwrap();

    let db = Arc::new(state.storage.clone());

    let res = db
        .lfs_delete_lock(
            &unlock_request.refs.name,
            None,
            &id,
            unlock_request.force.unwrap_or(false),
        )
        .await;

    let (deleted_lock, ok) = match res {
        Ok(lock) => (lock, true),
        Err(_) => (
            Lock {
                id: "".to_string(),
                path: "".to_string(),
                owner: None,
                locked_at: { DateTime::<Utc>::MIN_UTC.to_rfc3339().to_string() },
            },
            false,
        ),
    };

    if !ok {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            "Delete operation failed!".to_string(),
        ));
    }

    if deleted_lock.id == ""
        && deleted_lock.path == ""
        && deleted_lock.owner.is_none()
        && deleted_lock.locked_at == DateTime::<Utc>::MIN_UTC.to_rfc3339().to_string()
    {
        return Err((StatusCode::NOT_FOUND, "Unable to find lock!".to_string()));
    }

    let unlock_response = UnlockResponse {
        lock: deleted_lock,
        message: "".to_string(),
    };
    tracing::info!("sending: {:?}", unlock_response);
    let unlock_response = serde_json::to_string(&unlock_response).unwrap();

    let body = Body::from(unlock_response);
    Ok(resp.body(body).unwrap())
}

async fn lfs_process_batch<T>(
    state: State<AppState<T>>,
    req: Request<Body>,
) -> Result<Response<Body>, (StatusCode, String)>
where
    T: ObjectStorage,
{
    // Extract the body to `BatchVars`.
    tracing::info!("req: {:?}", req);

    let (_parts, mut body) = req.into_parts();

    let mut request_body = BytesMut::new();

    while let Some(chunk) = body.next().await {
        tracing::info!("client sends :{:?}", chunk);
        let bytes = chunk.unwrap();
        request_body.extend_from_slice(&bytes);
    }

    let mut batch_vars: BatchVars = serde_json::from_slice(request_body.freeze().as_ref()).unwrap();

    let bvo = &mut batch_vars.objects;
    for request in bvo {
        request.authorization = "".to_string();
    }
    tracing::info!("acquired: {:?}", batch_vars);

    let mut response_objects = Vec::<Representation>::new();

    let db = Arc::new(state.storage.clone());
    let config = Arc::new(state.config.clone());

    //
    let server_url = format!("http://{}:{}", config.host, config.port);

    let content_store = ContentStore::new(config.lfs_content_path.to_owned()).await;
    for object in batch_vars.objects {
        let meta = db.lfs_get_meta(&object).await;

        // Found
        let found = meta.is_ok();
        let mut meta = meta.unwrap_or_default();
        if found && content_store.exist(&meta).await {
            response_objects.push(represent(&object, &meta, true, false, false, &server_url).await);
            continue;
        }

        // Not found
        if batch_vars.operation == "upload" {
            meta = db.lfs_put_meta(&object).await.unwrap();
            response_objects.push(represent(&object, &meta, false, true, false, &server_url).await);
        } else {
            let rep = Representation {
                oid: object.oid.to_owned(),
                size: object.size,
                authenticated: None,
                actions: None,
                error: Some(ObjectError {
                    code: 404,
                    message: "Not found".to_owned(),
                }),
            };
            response_objects.push(rep);
        }
    }

    let batch_response = BatchResponse {
        transfer: "basic".to_string(),
        objects: response_objects,
        hash_algo: "sha256".to_string(),
    };

    let json = serde_json::to_string(&batch_response).unwrap();
    //DEBUG

    let mut resp = Response::builder();
    resp = resp.status(200);
    resp = resp.header("Content-Type", "application/vnd.git-lfs+json");

    let body = Body::from(json);
    let resp = resp.body(body).unwrap();
    println!("Sending: {:?}", resp);

    Ok(resp)
}

async fn lfs_upload_object<T>(
    state: State<AppState<T>>,
    oid: &str,
    req: Request<Body>,
) -> Result<Response<Body>, (StatusCode, String)>
where
    T: ObjectStorage,
{
    tracing::info!("req: {:?}", req);
    // Load request parameters into struct.
    let request_vars = RequestVars {
        oid: oid.to_string(),
        authorization: "".to_string(),
        ..Default::default()
    };

    let db = Arc::new(state.storage.clone());
    let config = Arc::new(state.config.clone());
    let content_store = ContentStore::new(config.lfs_content_path.to_owned()).await;

    let meta = db.lfs_get_meta(&request_vars).await.unwrap();

    let (_parts, mut body) = req.into_parts();

    let mut request_body = BytesMut::new();

    while let Some(chunk) = body.next().await {
        tracing::info!("client sends :{:?}", chunk);
        let bytes = chunk.unwrap();
        request_body.extend_from_slice(&bytes);
    }

    let ok = content_store
        .put(&meta, request_body.freeze().as_ref())
        .await;
    if !ok {
        db.lfs_delete_meta(&request_vars).await.unwrap();
        return Err((
            StatusCode::NOT_ACCEPTABLE,
            String::from("Header not acceptable!"),
        ));
    }
    let mut resp = Response::builder();
    resp = resp.header("Content-Type", "application/vnd.git-lfs");
    let resp = resp.body(Body::empty()).unwrap();

    Ok(resp)
}

async fn lfs_download_object<T>(
    state: State<AppState<T>>,
    oid: &str,
) -> Result<Response<Body>, (StatusCode, String)>
where
    T: ObjectStorage,
{
    tracing::info!("start downloading LFS object");
    let db = Arc::new(state.storage.clone());
    let config = Arc::new(state.config.clone());
    let content_store = ContentStore::new(config.lfs_content_path.to_owned()).await;

    // Load request parameters into struct.
    let request_vars = RequestVars {
        oid: oid.to_owned(),
        authorization: "".to_owned(),
        ..Default::default()
    };

    let meta = db.lfs_get_meta(&request_vars).await.unwrap();

    let mut file = content_store.get(&meta, 0).await;

    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).unwrap();
    let mut bytes = BytesMut::new();
    bytes.put(buffer.as_ref());
    let mut resp = Response::builder();
    resp = resp.status(200);
    let body = Body::from(bytes.freeze());
    Ok(resp.body(body).unwrap())
}

async fn represent(
    rv: &RequestVars,
    meta: &MetaObject,
    download: bool,
    upload: bool,
    use_tus: bool,
    server_url: &str,
) -> Representation {
    let mut rep = Representation {
        oid: meta.oid.to_owned(),
        size: meta.size,
        authenticated: Some(true),
        actions: None,
        error: None,
    };

    let mut header: HashMap<String, String> = HashMap::new();
    let mut verify_header: HashMap<String, String> = HashMap::new();

    header.insert("Accept".to_string(), "application/vnd.git-lfs".to_owned());

    if rv.authorization.len() > 0 {
        header.insert("Authorization".to_string(), rv.authorization.to_owned());
        verify_header.insert("Authorization".to_string(), rv.authorization.to_owned());
    }

    if download {
        let mut actions = HashMap::new();
        actions.insert(
            "download".to_string(),
            Link {
                href: { rv.download_link(server_url.to_string()).await },
                header: header.clone(),
                expires_at: {
                    let expire_time: DateTime<Utc> = Utc::now() + Duration::seconds(86400);
                    expire_time.to_rfc3339().to_string()
                },
            },
        );
        rep.actions = Some(actions);
    }

    if upload {
        let mut actions = HashMap::new();
        actions.insert(
            "upload".to_string(),
            Link {
                href: { rv.upload_link(server_url.to_string()).await },
                header: header.clone(),
                expires_at: {
                    let expire_time: DateTime<Utc> = Utc::now() + Duration::seconds(86400);
                    expire_time.to_rfc3339().to_string()
                },
            },
        );
        rep.actions = Some(actions);
        if use_tus {
            let mut actions = HashMap::new();
            actions.insert(
                "verify".to_string(),
                Link {
                    href: { rv.verify_link(server_url.to_string()).await },
                    header: verify_header.clone(),
                    expires_at: {
                        let expire_time: DateTime<Utc> = Utc::now() + Duration::seconds(86400);
                        expire_time.to_rfc3339().to_string()
                    },
                },
            );
            rep.actions = Some(actions);
        }
    }

    rep
}
