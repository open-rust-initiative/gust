use crate::git::object::metadata::MetaData;
use crate::git::object::types::ObjectType;
use crate::git::protocol::ProjectPath;
use async_trait::async_trait;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, InsertResult, QueryFilter,
};

use crate::git::object::base::commit::Commit;
use crate::git::object::base::BaseObject;
use crate::git::pack::decode::ObjDecodedMap;
use crate::git::pack::Pack;
use crate::gust::driver::database::entity::{commit, node, node_data};
use crate::gust::driver::structure::nodes::{build_node_tree, SaveModel};
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
    async fn get_head_object_id(&self, repo_path: &str) -> String {
        tracing::info!("{:?}", repo_path);
        let commits: Vec<commit::Model> = commit::Entity::find()
            .filter(commit::Column::RepoPath.eq(repo_path))
            .all(&self.connection)
            .await
            .unwrap();
        if commits.is_empty() {
            String::from_utf8_lossy(&[b'0'; 40]).to_string()
        } else {
            // todo filter one result
            commits[1].git_id.clone()
        }
    }

    fn search_child_objects(
        &self,
        _parent: Box<dyn BaseObject>,
    ) -> Result<Vec<Box<dyn BaseObject>>, anyhow::Error> {
        todo!()
    }

    async fn save_packfile(
        &self,
        decoded_pack: Pack,
        req_path: &str,
    ) -> Result<bool, anyhow::Error> {
        let mut result = ObjDecodedMap::default();
        result.update_from_cache(&decoded_pack.result);
        result.check_completeness().unwrap();

        let SaveModel { nodes, nodes_data } = build_node_tree(&result, req_path).await.unwrap();

        let node_result = self.save_nodes(nodes).await.unwrap();
        let data_result = self.save_node_data(nodes_data).await.unwrap();
        let commit_result = self.save_commits(&result.commits, req_path).await.unwrap();
        Ok(data_result && node_result && commit_result)
    }

    async fn get_full_pack_data(&self, path: &ProjectPath) -> Vec<u8> {
        let mut metadata_vec: Vec<MetaData> = Vec::new();
        let commits: Vec<commit::Model> = commit::Entity::find()
            .filter(commit::Column::RepoPath.eq(&path.repo_path))
            .all(&self.connection)
            .await
            .unwrap();
        for commit in commits.iter() {
            metadata_vec.push(MetaData::new(ObjectType::Commit, &commit.meta));
        }
        // TODO: added tree and blob

        let result: Vec<u8> = Pack::default().encode(Some(metadata_vec));
        result
    }

    async fn handle_pull_pack_data(&self, path: &ProjectPath) -> Vec<u8> {
        todo!();
    }
}

impl MysqlStorage {
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
            save_models.push(commit.convert_to_model(repo_path, &commit.meta.data));
        }
        batch_save_model(conn, save_models).await.unwrap();
        Ok(true)
    }

    async fn save_node_data(
        &self,
        save_models: Vec<node_data::ActiveModel>,
    ) -> Result<bool, anyhow::Error> {
        batch_save_model(&self.connection, save_models)
            .await
            .unwrap();
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
