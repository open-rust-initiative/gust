//!
//!
//！

use std::{
    collections::{HashMap, HashSet},
    path::Path,
};

use async_trait::async_trait;
use hyper::Request;

use crate::git::lfs::structs::*;
use crate::git::{
    errors::{GitError, GitLFSError},
    object::metadata::MetaData,
    pack::Pack,
    protocol::RefCommand,
};

pub mod database;
pub mod fs;
pub mod lfs_content_store;
pub mod structure;
pub mod utils;

pub const ZERO_ID: &'static str = match std::str::from_utf8(&[b'0'; 40]) {
    Ok(s) => s,
    Err(_) => panic!("can't get ZERO_ID"),
};

#[async_trait]
pub trait ObjectStorage: Clone + Send + Sync + std::fmt::Debug {
    async fn get_head_object_id(&self, path: &Path) -> String;

    async fn get_ref_object_id(&self, path: &Path) -> HashMap<String, String>;

    async fn handle_refs(&self, command: &RefCommand, path: &Path);

    async fn save_packfile(
        &self,
        decoded_pack: Pack,
        repo_path: &Path,
    ) -> Result<(), anyhow::Error>;

    async fn get_full_pack_data(&self, repo_path: &Path) -> Result<Vec<u8>, GitError>;

    async fn get_incremental_pack_data(
        &self,
        repo_path: &Path,
        want: &HashSet<String>,
        have: &HashSet<String>,
    ) -> Result<Vec<u8>, GitError>;

    async fn get_commit_by_hash(&self, hash: &str) -> Result<MetaData, GitError>;

    // get hash object from db if missing cache in unpack process, this object must be tree or blob
    async fn get_hash_object(&self, hash: &str) -> Result<MetaData, GitError>;

    async fn lfs_get_meta(&self, v: &RequestVars) -> Result<MetaObject, GitLFSError>;

    async fn lfs_put_meta(&self, v: &RequestVars) -> Result<MetaObject, GitLFSError>;

    async fn lfs_delete_meta(&self, v: &RequestVars) -> Result<(), GitLFSError>;

    async fn lfs_get_locks(&self, refspec: &str) -> Result<Vec<Lock>, GitLFSError>;

    async fn lfs_get_filtered_locks(
        &self,
        refspec: &str,
        path: &str,
        cursor: &str,
        limit: &str,
    ) -> Result<(Vec<Lock>, String), GitLFSError>;

    async fn lfs_add_lock(&self, refspec: &str, locks: Vec<Lock>) -> Result<(), GitLFSError>;

    async fn lfs_delete_lock(
        &self,
        refspec: &str,
        user: Option<String>,
        id: &str,
        force: bool,
    ) -> Result<Lock, GitLFSError>;
}
