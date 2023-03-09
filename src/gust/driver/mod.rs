//!
//!

use std::path::Path;

use async_trait::async_trait;

use crate::git::object::base::BaseObject;

use self::database::entity::node;
//！
pub mod database;
pub mod fs;
pub mod structure;
pub mod utils;

// #[derive(Clone)]
// pub enum StorageType {
//     Mysql(MysqlStorage),
//     Filesystem,
// }

#[async_trait]
pub trait ObjectStorage {
    fn get_head_object_id(&self, work_dir: &Path) -> String;

    fn search_child_objects(
        &self,
        parent: Box<dyn BaseObject>,
    ) -> Result<Vec<Box<dyn BaseObject>>, anyhow::Error>;

    async fn persist_node_objects(
        &self,
        objects: Vec<node::ActiveModel>,
    ) -> Result<bool, anyhow::Error>;
}
