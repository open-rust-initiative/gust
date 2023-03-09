use std::path::Path;

use async_trait::async_trait;
use sea_orm::ActiveValue::NotSet;
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, InsertResult, Set};

use crate::git::object::base::blob::Blob;
use crate::git::object::base::tree::Tree;
use crate::git::object::base::BaseObject;
use crate::gust::driver::database::entity::node;
use crate::gust::driver::utils::id_generator;
use crate::gust::driver::ObjectStorage;

#[derive(Debug, Default, Clone)]
pub struct MysqlStorage {
    pub connection: DatabaseConnection,
}

impl MysqlStorage {
    pub fn new(connection: DatabaseConnection) -> MysqlStorage {
        MysqlStorage { connection }
    }
}

#[async_trait]
impl ObjectStorage for MysqlStorage {
    fn get_head_object_id(&self, work_dir: &Path) -> String {
        // TODO update to mysql logic
        let content = std::fs::read_to_string(work_dir.join("HEAD")).unwrap();
        let content = content.replace("ref: ", "");
        let content = content.strip_suffix('\n').unwrap();
        let object_id = match std::fs::read_to_string(work_dir.join(content)) {
            Ok(object_id) => object_id.strip_suffix('\n').unwrap().to_owned(),
            _ => String::from_utf8_lossy(&[b'0'; 40]).to_string(),
        };
        object_id
    }

    fn search_child_objects(
        &self,
        _parent: Box<dyn BaseObject>,
    ) -> Result<Vec<Box<(dyn BaseObject + 'static)>>, anyhow::Error> {
        todo!()
    }

    async fn persist_node_objects(
        &self,
        objects: Vec<node::ActiveModel>,
    ) -> Result<bool, anyhow::Error> {
        let conn = &self.connection;
        batch_save_model(conn, objects).await.unwrap();
        Ok(true)
    }
}

pub trait GitNodeObject {
    fn convert_to_model(&self) -> node::ActiveModel;

    fn generate_id() -> i64 {
        id_generator::generate_id()
    }
}

impl GitNodeObject for Tree {
    fn convert_to_model(&self) -> node::ActiveModel {
        // tracing::debug!("{}", self.meta.id.to_string());
        node::ActiveModel {
            id: NotSet,
            node_id: Set(Self::generate_id()),
            object_id: Set(self.meta.id.to_plain_str()),
            node_type: Set("tree".to_owned()),
            content_sha1: NotSet,
            name: Set(Some(self.tree_name.to_string())),
            path: NotSet,
            create_time: NotSet,
            update_time: NotSet,
        }
    }
}

impl GitNodeObject for Blob {
    fn convert_to_model(&self) -> node::ActiveModel {
        node::ActiveModel {
            id: NotSet,
            node_id: Set(Self::generate_id()),
            object_id: Set(self.meta.id.to_plain_str()),
            node_type: Set("blob".to_owned()),
            content_sha1: NotSet,
            name: Set(Some(self.filename.to_string())),
            path: NotSet,
            create_time: NotSet,
            update_time: NotSet,
        }
    }
}

// mysql sea_orm bathc insert
async fn batch_save_model<E, A>(
    conn: &DatabaseConnection,
    save_models: Vec<A>,
) -> Result<Vec<InsertResult<A>>, anyhow::Error>
where
    E: EntityTrait,
    A: ActiveModelTrait<Entity = E> + From<<E as EntityTrait>::Model> + Send,
{
    let mut result_vec = Vec::new();
    for chunk in save_models.chunks(1000) {
        let save_result = E::insert_many(chunk.iter().cloned())
            .exec(conn)
            .await
            .unwrap();
        // println!("{:?}", save_result);
        result_vec.push(save_result);
    }
    Ok(result_vec)
}
