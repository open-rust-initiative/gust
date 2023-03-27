use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::git::object::base::commit::Commit;
use crate::git::object::metadata::MetaData;
use crate::git::object::types::ObjectType;
use crate::git::pack::decode::ObjDecodedMap;
use crate::git::pack::Pack;
use crate::gust::driver::database::entity::{commit, node, node_data};
use crate::gust::driver::structure::nodes::{build_node_tree, model_to_tree, SaveModel};
use crate::gust::driver::ObjectStorage;
use async_trait::async_trait;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseBackend, DatabaseConnection, EntityTrait, InsertResult,
    QueryFilter, Statement,
};

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
    async fn get_head_object_id(&self, repo_path: &PathBuf) -> String {
        let path_str = repo_path.to_str().unwrap();
        // consider a search condition: root/repotest2/src
        let commits: Vec<commit::Model> = commit::Entity::find()
            .from_raw_sql(Statement::from_sql_and_values(
                DatabaseBackend::MySql,
                r#"SELECT * FROM gust.commit where ? LIKE CONCAT(repo_path, '%')"#,
                [path_str.into()],
            ))
            .all(&self.connection)
            .await
            .unwrap();
        tracing::debug!("path_str: {}", path_str);
        if commits.is_empty() {
            String::from_utf8_lossy(&[b'0'; 40]).to_string()
        } else {
            for commit in &commits {
                if repo_path.to_str().unwrap() == commit.repo_path {
                    return commit.git_id.clone();
                }
            }
            for commit in &commits {
                // repo_path is subdirectory of some commit
                if repo_path.starts_with(commit.repo_path.clone()) {
                    let fake_commit = self.generate_fake_commit(commit, repo_path).await;
                    return fake_commit.meta.id.to_plain_str();
                }
            }
            //situation: repo_path: root/repotest2/src, commit: root/repotest
            String::from_utf8_lossy(&[b'0'; 40]).to_string()
        }
    }

    async fn get_ref_object_id(&self, repo_path: &PathBuf) -> HashMap<String, String> {
        HashMap::new()
    }

    async fn save_packfile(
        &self,
        decoded_pack: Pack,
        repo_path: &PathBuf,
    ) -> Result<bool, anyhow::Error> {
        let mut result = ObjDecodedMap::default();
        result.update_from_cache(&decoded_pack.result);
        result.check_completeness().unwrap();

        let SaveModel { nodes, nodes_data } = build_node_tree(&result, repo_path).await.unwrap();

        let node_result = self.save_nodes(nodes).await.unwrap();
        let data_result = self.save_node_data(nodes_data).await.unwrap();
        let commit_result = self.save_commits(&result.commits, repo_path).await.unwrap();
        Ok(data_result && node_result && commit_result)
    }

    async fn get_full_pack_data(&self, repo_path: &PathBuf) -> Vec<u8> {
        let mut metadata_vec: Vec<MetaData> = Vec::new();
        let blob_models: Vec<node_data::Model> = node_data::Entity::find()
            .all(&self.connection)
            .await
            .unwrap();
        for b in blob_models {
            metadata_vec.push(MetaData::new(ObjectType::Blob, &b.data));
        }
        let node_models: Vec<node::Model> = node::Entity::find()
            .filter(node::Column::Path.contains(repo_path.to_str().unwrap()))
            .all(&self.connection)
            .await
            .unwrap();
        tracing::debug!("repo_path: {:?}", repo_path);

        let root = self.search_root_by_path(repo_path).await.unwrap();
        model_to_tree(&node_models, &root, &mut metadata_vec);

        let commit: commit::Model = commit::Entity::find()
            .filter(commit::Column::RepoPath.eq(repo_path.to_str()))
            .one(&self.connection)
            .await
            .unwrap()
            .unwrap();
        metadata_vec.push(MetaData::new(ObjectType::Commit, &commit.meta));

        let result: Vec<u8> = Pack::default().encode(Some(metadata_vec));
        result
    }

    async fn handle_pull_pack_data(&self) -> Vec<u8> {
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
        repo_path: &Path,
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

    /// Because the requested path is a subdirectory of the original project directory,
    /// a new fake commit is needed that points to this subdirectory, so we need to
    /// 1. convert commit to git Commit object and calculate it's hash
    /// 2. save the new fake commit with hash and repo_path
    async fn generate_fake_commit(&self, model: &commit::Model, repo_path: &Path) -> Commit {
        let root = self.search_root_by_path(repo_path).await.unwrap();
        let fake_commit = Commit::build_from_model_and_root(model, root);
        self.save_commits(&vec![fake_commit.clone()], repo_path)
            .await
            .unwrap();
        fake_commit
    }

    async fn search_root_by_path(&self, repo_path: &Path) -> Option<node::Model> {
        tracing::debug!("file_name: {:?}", repo_path.file_name());
        node::Entity::find()
            // .filter(node::Column::Path.like(repo_path.to_str().unwrap()))
            .filter(node::Column::Name.eq(repo_path.file_name().unwrap().to_str().unwrap()))
            .one(&self.connection)
            .await
            .unwrap()
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
