use sea_orm::ActiveValue::NotSet;
use sea_orm::{ActiveModelTrait, EntityTrait, InsertResult, Set};

use crate::database::entity::{object_content, object_info};
use crate::database::DataSource;
use crate::git::object::base::BaseObject;
use crate::gust::driver::database::object_service::ObjectService;

// #[derive(Debug, Default)]
pub struct Mysql<'a> {
    datasource: &'a DataSource,
}

impl ObjectService for Mysql<'_> {
    fn search_child_objects(&self, parent: Box<dyn BaseObject>) -> Vec<Box<dyn BaseObject>> {
        todo!()
    }

    fn save_objects(&self, objects: Vec<Box<dyn BaseObject>>) {
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
                obj_type: Set(object.get_object_type().to_string()),
            };
            infos.push(object_info)
        }
        // write data to multiple tables
        let res = async {
            save_objects(self.datasource, contents).await.unwrap();
            save_objects(self.datasource, infos).await.unwrap();
            "ok"
        };
    }
}

// mysql sea_orm bathc insert
async fn save_objects<E, A>(
    data_source: &DataSource,
    save_models: Vec<A>,
) -> Result<Vec<InsertResult<A>>, anyhow::Error>
where
    E: EntityTrait,
    A: ActiveModelTrait<Entity = E> + From<<E as EntityTrait>::Model> + Send,
{
    let mut result_vec = Vec::new();
    for chunk in save_models.chunks(1000) {
        let save_result = E::insert_many(chunk.iter().cloned())
            .exec(&data_source.sea_orm)
            .await
            .unwrap();
        result_vec.push(save_result);
    }
    Ok(result_vec)
}
