use std::path::Path;

use async_trait::async_trait;
use sea_orm::ActiveValue::NotSet;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, InsertResult, QueryFilter, Set,
};

use crate::git::object::base::commit::Commit;
use crate::git::object::base::BaseObject;
use crate::gust::driver::database::entity::{commit, node};
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
    async fn get_head_object_id(&self, repo_path: &Path) -> String {
        let commit: commit::Model = commit::Entity::find()
            .filter(commit::Column::IsHead.eq(true))
            .one(&self.connection)
            .await
            .unwrap()
            .unwrap();
        commit.oid
    }

    fn search_child_objects(
        &self,
        _parent: Box<dyn BaseObject>,
    ) -> Result<Vec<Box<dyn BaseObject>>, anyhow::Error> {
        todo!()
    }

    async fn save_nodes(&self, objects: Vec<node::ActiveModel>) -> Result<bool, anyhow::Error> {
        let conn = &self.connection;
        batch_save_model(conn, objects).await.unwrap();
        Ok(true)
    }

    async fn save_commits(
        &self,
        commits: &Vec<Commit>,
        repo_path: &str,
    ) -> Result<bool, anyhow::Error> {
        let conn = &self.connection;
        let mut save_models: Vec<commit::ActiveModel> = Vec::new();
        for commit in commits {
            let commit_model = commit::ActiveModel {
                id: NotSet,
                oid: Set(commit.meta.id.to_plain_str()),
                tree: Set(commit.tree_id.to_plain_str()),
                pid: NotSet,
                is_head: Set(false),
                repo_path: Set(repo_path.to_string()),
                author: NotSet,
                committer: NotSet,
                content: NotSet,
                created_at: Set(chrono::Utc::now().naive_utc()),
                updated_at: Set(chrono::Utc::now().naive_utc()),
            };
            save_models.push(commit_model);
        }
        tracing::info!("Commits saved{:?}", save_models);
        batch_save_model(conn, save_models).await.unwrap();
        Ok(true)
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
