//!
//!

use async_trait::async_trait;

use crate::gateway::api::lib::StorageType; 
use crate::{git::object::base::BaseObject};
//！
pub mod database;

#[async_trait]
pub trait ObjectStorage {
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

#[derive(Default)]
pub struct BasicObject {
    pub file: String,
    pub hash: String,

}
