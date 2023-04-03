//!
//!
//！

use std::{collections::HashMap, path::PathBuf};

use async_trait::async_trait;

use crate::git::{pack::Pack, protocol::RefCommand};

pub mod database;
pub mod fs;
pub mod structure;
pub mod utils;

pub const ZERO_ID: &'static str = match std::str::from_utf8(&[b'0'; 40]) {
    Ok(s) => s,
    Err(_) => panic!("can't get ZERO_ID"),
};

#[async_trait]
pub trait ObjectStorage: Clone + Send + Sync {
    async fn get_head_object_id(&self, path: &PathBuf) -> String;

    async fn get_ref_object_id(&self, path: &PathBuf) -> HashMap<String, String>;

    async fn handle_refs(&self, command: &RefCommand, path: &PathBuf);

    async fn save_packfile(
        &self,
        decoded_pack: Pack,
        repo_path: &PathBuf,
    ) -> Result<bool, anyhow::Error>;

    async fn get_full_pack_data(&self, repo_path: &PathBuf) -> Vec<u8>;

    async fn handle_pull_pack_data(&self) -> Vec<u8>;
}
