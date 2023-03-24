//!
//!
//！

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use async_trait::async_trait;

use crate::git::pack::Pack;

pub mod database;
pub mod fs;
pub mod structure;
pub mod utils;

#[async_trait]
pub trait ObjectStorage {
    async fn get_head_object_id(&self, path: &PathBuf) -> String;

    async fn get_ref_object_id(&self, path: &PathBuf) -> HashMap<String, String>;

    async fn save_packfile(
        &self,
        decoded_pack: Pack,
        repo_path: &PathBuf,
    ) -> Result<bool, anyhow::Error>;

    async fn get_full_pack_data(&self, repo_path: &PathBuf) -> Vec<u8>;

    async fn handle_pull_pack_data(&self) -> Vec<u8>;

    // fn get_path(&self) -> PathBuf;

    // fn set_path(&mut self, params: Params);
}
