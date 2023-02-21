use std::path::PathBuf;

use async_trait::async_trait;
use sea_orm::ActiveValue::NotSet;
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, InsertResult, Set};

use crate::git::object::base::BaseObject;
use crate::gust::driver::database::entity::{object_content, object_info};
use crate::gust::driver::{BasicObject, ObjectStorage, StorageType};

#[derive(Debug, Default)]
pub struct MysqlStorage {
    pub connection: DatabaseConnection,
}

impl MysqlStorage {
    pub fn new(storage: &StorageType) -> MysqlStorage {
        if let StorageType::Mysql(connection) = storage {
            return MysqlStorage {
                connection: connection.to_owned(),
            };
        } else {
            panic!("Not supported storage type");
        };
    }
}

#[async_trait]
impl ObjectStorage for MysqlStorage {
    fn get_head_object_id(&self, work_dir: &PathBuf) -> String {
        // TODO update to mysql logic
        let content = std::fs::read_to_string(work_dir.join("HEAD")).unwrap();
        let content = content.replace("ref: ", "");
        let content = content.strip_suffix('\n').unwrap();
        let object_id = match std::fs::read_to_string(work_dir.join(content)) {
            Ok(object_id) => object_id.strip_suffix('\n').unwrap().to_owned(),
            _ => String::from_utf8_lossy(&vec![b'0'; 40]).to_string()
        };
        object_id
    }

    fn search_child_objects(
        &self,
        _storage: &StorageType,
        _parent: Box<dyn BaseObject>,
    ) -> Result<Vec<Box<(dyn BaseObject + 'static)>>, anyhow::Error> {
        todo!()
    }

    async fn save_objects(
        &self,
        _storage: &StorageType,
        objects: Vec<BasicObject>,
    ) -> Result<bool, anyhow::Error> {
        let conn = &self.connection;
        let mut contents = Vec::new();
        let mut infos = Vec::new();
        // converting types
        for _object in objects {
            let object_content = object_content::ActiveModel {
                id: NotSet,
                file: Set("file".to_owned()),
                hash: Set("hash".to_owned()),
            };
            contents.push(object_content);
            let object_info = object_info::ActiveModel {
                id: NotSet,
                hash: Set("hash".to_owned()),
                path: Set("path".to_owned()),
                obj_type: Set("commit".to_owned()),
            };
            infos.push(object_info)
        }
        save_objects(conn, contents).await.unwrap();
        save_objects(conn, infos).await.unwrap();
        Ok(true)
    }
}

// mysql sea_orm bathc insert
async fn save_objects<E, A>(
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
