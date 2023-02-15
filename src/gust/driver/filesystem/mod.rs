use std::path::PathBuf;

use async_trait::async_trait;

use crate::git::object::base::BaseObject;

use super::{ObjectStorage, StorageType};

pub mod nodes;

pub struct FileSystem;

#[async_trait]
impl ObjectStorage for FileSystem {
    fn get_head_object_id(&self, work_dir: &PathBuf) -> String {
        let content = std::fs::read_to_string(work_dir.join("HEAD")).unwrap();
        let content = content.replace("ref: ", "");
        let content = content.strip_suffix('\n').unwrap();
        let object_id = std::fs::read_to_string(work_dir.join(content)).unwrap();
        let object_id = object_id.strip_suffix('\n').unwrap();
        object_id.to_owned()
    }

    fn search_child_objects(
        &self,
        storage: &StorageType,
        parent: Box<dyn BaseObject>,
    ) -> Result<Vec<Box<dyn BaseObject>>, anyhow::Error> {
        todo!()
    }

    async fn save_objects(
        &self,
        storage: &StorageType,
        objects: Vec<crate::gust::driver::BasicObject>,
    ) -> Result<bool, anyhow::Error> {
        todo!()
    }
}
