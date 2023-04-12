use std::collections::HashMap;
use std::path::Path;

use crate::git::object::base::commit::Commit;
use crate::git::object::metadata::MetaData;
use crate::git::object::types::ObjectType;
use crate::git::pack::decode::ObjDecodedMap;
use crate::git::pack::Pack;
use crate::git::protocol::{Command, RefCommand};
use crate::gust::driver::database::entity::{commit, node, node_data, refs};
use crate::gust::driver::structure::nodes::{build_node_tree, model_to_tree, SaveModel};
use crate::gust::driver::{ObjectStorage, ZERO_ID};
use async_trait::async_trait;
use sea_orm::ActiveValue::NotSet;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseBackend, DatabaseConnection, DbErr, EntityTrait,
    InsertResult, QueryFilter, Set, Statement,
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
    async fn get_head_object_id(&self, repo_path: &Path) -> String {
        let path_str = repo_path.to_str().unwrap();
        // consider a search condition: root/repotest2/src
        // let commits: Vec<commit::Model> = commit::Entity::find()
        //     .from_raw_sql(Statement::from_sql_and_values(
        //         DatabaseBackend::MySql,
        //         r#"SELECT * FROM gust.commit where ? LIKE CONCAT(repo_path, '%')"#,
        //         [path_str.into()],
        //     ))
        //     .all(&self.connection)
        //     .await
        //     .unwrap();
        let refs_list = self.search_refs(path_str).await.unwrap();

        if refs_list.is_empty() {
            ZERO_ID.to_string()
        } else {
            for refs in &refs_list {
                if repo_path.to_str().unwrap() == refs.repo_path {
                    return refs.ref_git_id.clone();
                }
            }
            for refs in &refs_list {
                // repo_path is subdirectory of some commit
                if repo_path.starts_with(refs.repo_path.clone()) {
                    return self.generate_child_commit_and_refs(refs, repo_path).await;
                }
            }
            //situation: repo_path: root/repotest2/src, commit: root/repotest
            ZERO_ID.to_string()
        }
    }

    async fn get_ref_object_id(&self, repo_path: &Path) -> HashMap<String, String> {
        // assuming HEAD points to branch master.
        let mut map = HashMap::new();
        let refs: Vec<refs::Model> = refs::Entity::find()
            .filter(refs::Column::RepoPath.eq(repo_path.to_str()))
            .all(&self.connection)
            .await
            .unwrap();
        for git_ref in refs {
            map.insert(git_ref.ref_git_id, git_ref.ref_name);
        }
        map
    }

    async fn handle_refs(&self, command: &RefCommand, path: &Path) {
        match command.command_type {
            Command::Create => self.save_refs(command, path).await,
            Command::Delete => self.delete_refs(command, path).await,
            Command::Update => self.update_refs(command, path).await,
        }
    }

    async fn save_packfile(
        &self,
        decoded_pack: Pack,
        repo_path: &Path,
    ) -> Result<(), anyhow::Error> {
        let mut result = ObjDecodedMap::default();
        result.update_from_cache(&decoded_pack.result);
        // result.check_completeness()?;
        let SaveModel { nodes, nodes_data } = build_node_tree(&result, repo_path).await.unwrap();
        self.save_nodes(nodes).await.unwrap();
        self.save_node_data(nodes_data).await.unwrap();
        self.save_commits(&result.commits, repo_path).await.unwrap();
        Ok(())
    }

    async fn get_full_pack_data(&self, repo_path: &Path) -> Vec<u8> {
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
    async fn search_refs(&self, path_str: &str) -> Result<Vec<refs::Model>, DbErr> {
        refs::Entity::find()
        .from_raw_sql(Statement::from_sql_and_values(
            DatabaseBackend::MySql,
            r#"SELECT * FROM gust.refs where ? LIKE CONCAT(repo_path, '%') and ref_name = 'refs/heads/master' "#,
            [path_str.into()],
        ))
        .all(&self.connection)
        .await
    }

    async fn save_refs(&self, command: &RefCommand, path: &Path) {
        let mut save_models: Vec<refs::ActiveModel> = vec![];
        save_models.push(command.convert_to_model(path.to_str().unwrap()));
        batch_save_model(&self.connection, save_models)
            .await
            .unwrap();
    }

    async fn update_refs(&self, command: &RefCommand, path: &Path) {
        let ref_data: Option<refs::Model> = refs::Entity::find()
            .filter(refs::Column::RefGitId.eq(&command.old_id))
            .filter(refs::Column::RepoPath.eq(path.to_str().unwrap()))
            .one(&self.connection)
            .await
            .unwrap();
        let mut ref_data: refs::ActiveModel = ref_data.unwrap().into();
        ref_data.ref_git_id = Set(command.new_id.to_owned());
        ref_data.update(&self.connection).await.unwrap();
    }

    async fn delete_refs(&self, command: &RefCommand, path: &Path) {
        let delete_ref = refs::ActiveModel {
            ref_git_id: Set(command.old_id.to_owned()),
            repo_path: Set(path.to_str().unwrap().to_owned()),
            ..Default::default()
        };
        refs::Entity::delete(delete_ref)
            .exec(&self.connection)
            .await
            .unwrap();
    }

    async fn search_commits(&self, path_str: &str) -> Result<Vec<commit::Model>, DbErr> {
        commit::Entity::find()
            .from_raw_sql(Statement::from_sql_and_values(
                DatabaseBackend::MySql,
                r#"SELECT * FROM gust.commit where ? LIKE CONCAT(repo_path, '%')"#,
                [path_str.into()],
            ))
            .all(&self.connection)
            .await
    }

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
    /// 1. find root commit by root_ref
    /// 2. convert commit to git Commit object and calculate it's hash
    /// 3. save the new fake commit with hash and repo_path
    async fn generate_child_commit_and_refs(&self, refs: &refs::Model, repo_path: &Path) -> String {
        let root_commit = commit::Entity::find()
            .filter(commit::Column::GitId.eq(&refs.ref_git_id))
            .one(&self.connection)
            .await
            .unwrap()
            .unwrap();

        let root = self.search_root_by_path(repo_path).await.unwrap();
        let child_commit = Commit::build_from_model_and_root(&root_commit, root);
        self.save_commits(&vec![child_commit.clone()], repo_path)
            .await
            .unwrap();
        let commit_id = child_commit.meta.id.to_plain_str();
        let child_refs = refs::ActiveModel {
            id: NotSet,
            repo_path: Set(repo_path.to_str().unwrap().to_string()),
            ref_name: Set(refs.ref_name.clone()),
            ref_git_id: Set(commit_id.clone()),
            created_at: Set(chrono::Utc::now().naive_utc()),
            updated_at: Set(chrono::Utc::now().naive_utc()),
        };
        batch_save_model(&self.connection, vec![child_refs])
            .await
            .unwrap();

        commit_id
    }

    async fn search_root_by_path(&self, repo_path: &Path) -> Option<node::Model> {
        tracing::debug!("file_name: {:?}", repo_path.file_name());
        let res = node::Entity::find()
            .filter(node::Column::Name.eq(repo_path.file_name().unwrap().to_str().unwrap()))
            .one(&self.connection)
            .await
            .unwrap();
        if let Some(res) = res {
            Some(res)
        } else {
            node::Entity::find()
                .filter(node::Column::Path.eq(repo_path.to_str().unwrap()))
                .filter(node::Column::Name.eq(""))
                .one(&self.connection)
                .await
                .unwrap()
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
