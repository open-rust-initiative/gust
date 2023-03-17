//!
//!
//！

use async_trait::async_trait;

use crate::git::{object::base::BaseObject, pack::Pack, protocol::ProjectPath};

pub mod database;
pub mod fs;
pub mod structure;
pub mod utils;

#[async_trait]
pub trait ObjectStorage {
    async fn get_head_object_id(&self, repo_path: &str) -> String;

    fn search_child_objects(
        &self,
        parent: Box<dyn BaseObject>,
    ) -> Result<Vec<Box<dyn BaseObject>>, anyhow::Error>;

    async fn save_packfile(
        &self,
        decoded_pack: Pack,
        req_path: &str,
    ) -> Result<bool, anyhow::Error>;

    async fn get_full_pack_data(&self, path: &ProjectPath) -> Vec<u8>;

    async fn handle_pull_pack_data(&self, path: &ProjectPath) -> Vec<u8>;
}
