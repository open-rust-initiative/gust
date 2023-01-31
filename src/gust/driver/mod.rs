//!
//!

use crate::{git::object::base::BaseObject, gateway::api::lib::StorageType};
//！
pub mod database;
pub mod filesystem;



pub trait ObjectStorage {
    fn search_child_objects(
        &self,
        storage: &StorageType,
        parent: Box<dyn BaseObject>,
    ) -> Result<Vec<Box<dyn BaseObject>>, anyhow::Error>;

    fn save_objects(&self, storage: &StorageType, objects: Vec<BasicObject>) -> Result<bool, anyhow::Error>;
}

#[derive(Default)]
pub struct BasicObject {
    pub file: String,
    pub hash: String,

}
