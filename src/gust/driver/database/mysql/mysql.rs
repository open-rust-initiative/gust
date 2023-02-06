use async_trait::async_trait;
use sea_orm::ActiveValue::NotSet;
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, InsertResult, Set};

use crate::gateway::api::lib::StorageType;
use crate::git::object::base::BaseObject;
use crate::gust::driver::database::entity::{object_content, object_info};
use crate::gust::driver::{BasicObject, ObjectStorage};

#[derive(Debug, Default)]
pub struct Mysql {}

#[async_trait]
impl ObjectStorage for Mysql {
    fn search_child_objects(
        &self,
        storage: &StorageType,
        parent: Box<dyn BaseObject>,
    ) -> Result<Vec<Box<(dyn BaseObject + 'static)>>, anyhow::Error> {
        todo!()
    }

    async fn save_objects(
        &self,
        storage: &StorageType,
        objects: Vec<BasicObject>,
    ) -> Result<bool, anyhow::Error> {
        let conn = check_storage(storage);
        let mut contents = Vec::new();
        let mut infos = Vec::new();
        // converting types
        for object in objects {
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
                // obj_type: Set(object.get_object_type().to_string()),
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

fn check_storage(storage: &StorageType) -> &DatabaseConnection {
    if let StorageType::Mysql(conn) = storage {
        return conn;
    } else {
        panic!("Not supported storage type");
    };
}
