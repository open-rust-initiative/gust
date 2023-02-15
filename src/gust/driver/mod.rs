//!
//!

use std::path::PathBuf;

use async_trait::async_trait;
use sea_orm::DatabaseConnection;

use crate::git::object::base::BaseObject;
//！
pub mod database;
pub mod filesystem;

#[derive(Clone)]
pub enum StorageType {
    Mysql(DatabaseConnection),
    Filesystem,
}

#[derive(Default)]
pub struct BasicObject {
    pub file: String,
    pub hash: String,
}

#[async_trait]
pub trait ObjectStorage {
    fn get_head_object_id(&self, work_dir: &PathBuf) -> String;

    fn search_child_objects(
        &self,
        storage: &StorageType,
        parent: Box<dyn BaseObject>,
    ) -> Result<Vec<Box<dyn BaseObject>>, anyhow::Error>;

    async fn save_objects(
        &self,
        storage: &StorageType,
        objects: Vec<BasicObject>,
    ) -> Result<bool, anyhow::Error>;
}
