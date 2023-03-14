//!
//!
//！

use std::path::Path;

use async_trait::async_trait;

use crate::git::object::base::{commit::Commit, BaseObject};

use self::database::entity::node;

pub mod database;
pub mod fs;
pub mod structure;
pub mod utils;

#[async_trait]
pub trait ObjectStorage {
    async fn get_head_object_id(&self, repo_path: &Path) -> String;

    fn search_child_objects(
        &self,
        parent: Box<dyn BaseObject>,
    ) -> Result<Vec<Box<dyn BaseObject>>, anyhow::Error>;

    async fn save_nodes(&self, objects: Vec<node::ActiveModel>) -> Result<bool, anyhow::Error>;

    async fn save_commits(
        &self,
        commits: &Vec<Commit>,
        repo_path: &str,
    ) -> Result<bool, anyhow::Error>;
}
