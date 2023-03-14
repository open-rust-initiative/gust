use std::path::PathBuf;

use async_trait::async_trait;

use crate::{
    git::object::base::BaseObject,
    gust::driver::{database::entity::node, ObjectStorage, Path},
};

#[derive(Debug, Default, Clone)]
pub struct FileSystem {
    pub work_dir: PathBuf
}

#[async_trait]
impl ObjectStorage for FileSystem {
    fn get_head_object_id(&self) -> String {
        let content = std::fs::read_to_string(self.work_dir.join("HEAD")).unwrap();
        let content = content.replace("ref: ", "");
        let content = content.strip_suffix('\n').unwrap();
        let object_id = match std::fs::read_to_string(self.work_dir.join(content)) {
            Ok(object_id) => object_id.strip_suffix('\n').unwrap().to_owned(),
            _ => String::from_utf8_lossy(&[b'0'; 40]).to_string(),
        };
        object_id
    }

    fn search_child_objects(
        &self,
        // storage: &StorageType,
        parent: Box<dyn BaseObject>,
    ) -> Result<Vec<Box<dyn BaseObject>>, anyhow::Error> {
        todo!()
    }
    async fn persist_node_objects(
        &self,
        objects: Vec<node::ActiveModel>,
    ) -> Result<bool, anyhow::Error> {
        todo!()
    }
}
