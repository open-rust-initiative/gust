use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::git::errors::GitError;
use crate::git::object::base::commit::Commit;
use crate::git::object::base::tree::Tree;
use crate::git::object::metadata::MetaData;
use crate::git::object::types::ObjectType;
use crate::git::pack::decode::ObjDecodedMap;
use crate::git::pack::Pack;
use crate::git::protocol::{Command, RefCommand};
use crate::gust::driver::database::entity::{commit, node, refs};
use crate::gust::driver::structure::nodes::build_node_tree;
use crate::gust::driver::{ObjectStorage, ZERO_ID};
use async_recursion::async_recursion;
use async_trait::async_trait;
use sea_orm::ActiveValue::NotSet;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseBackend, DatabaseConnection, DbErr, EntityTrait,
    QueryFilter, Set, Statement,
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
        let nodes = build_node_tree(&result, repo_path).await.unwrap();
        self.save_nodes(nodes).await.unwrap();
        self.save_commits(&result.commits, repo_path).await.unwrap();
        Ok(())
    }

    async fn get_full_pack_data(&self, repo_path: &Path) -> Result<Vec<u8>, GitError> {
        let mut hash_meta: HashMap<String, MetaData> = HashMap::new();

        let commits = self.get_all_commits_by_path(repo_path).await.unwrap();
        for c_meta in commits {
            let c = Commit::new(c_meta);
            hash_meta.insert(c.meta.id.to_plain_str(), c.meta);
            tracing::info!("{}", c.tree_id.to_plain_str());
            if let Some(root) = self.get_node_by_id(&c.tree_id.to_plain_str()).await {
                self.get_child_trees(&root, &mut hash_meta).await
            } else {
                return Err(GitError::InvalidTreeObject(c.tree_id.to_plain_str()));
            };
        }
        let result: Vec<u8> = Pack::default().encode(Some(hash_meta.into_values().collect()));
        Ok(result)
    }

    async fn get_incremental_pack_data(
        &self,
        repo_path: &Path,
        want: &HashSet<String>,
        _have: &HashSet<String>,
    ) -> Result<Vec<u8>, GitError> {
        let mut hash_meta: HashMap<String, MetaData> = HashMap::new();
        let all_commits = self.get_all_commits_by_path(repo_path).await.unwrap();

        for c_meta in all_commits {
            if want.contains(&c_meta.id.to_plain_str()) {
                let c = Commit::new(c_meta);
                if let Some(root) = self.get_node_by_id(&c.tree_id.to_plain_str()).await {
                    self.get_child_trees(&root, &mut hash_meta).await
                } else {
                    return Err(GitError::InvalidTreeObject(c.tree_id.to_plain_str()));
                };
            }
        }

        let result: Vec<u8> = Pack::default().encode(Some(hash_meta.into_values().collect()));
        Ok(result)
    }

    async fn get_commit_by_hash(&self, hash: &str) -> Result<MetaData, GitError> {
        let commit: Option<commit::Model> = commit::Entity::find()
            .filter(commit::Column::GitId.eq(hash))
            .one(&self.connection)
            .await
            .unwrap();
        if let Some(commit) = commit {
            Ok(MetaData::new(ObjectType::Commit, &commit.meta))
        } else {
            return Err(GitError::InvalidCommitObject(hash.to_string()));
        }
    }

    async fn get_hash_object(&self, hash: &str) -> Result<MetaData, GitError> {
        tracing::info!("hash:{}", hash);
        let model = node::Entity::find()
            .filter(node::Column::GitId.eq(hash))
            .one(&self.connection)
            .await
            .unwrap();

        if let Some(model) = model {
            if model.node_type == "tree" {
                // let mut tree_items: Vec<TreeItem> = Vec::new();
                // let childs = node::Entity::find()
                //     .filter(node::Column::Pid.eq(hash))
                //     .all(&self.connection)
                //     .await
                //     .unwrap();
                // for c in childs {
                //     tree_items.push(TreeItem::convert_from_model(c));
                // }
                // let t = Tree::convert_from_model(&model, tree_items);
                // let meta = t.encode_metadata().unwrap();
                Ok(MetaData::new(ObjectType::Tree, &model.data))
            } else {
                Ok(MetaData::new(ObjectType::Blob, &model.data))
            }
        } else {
            return Err(GitError::NotFountHashValue(hash.to_string()));
        }
    }
}

impl MysqlStorage {
    async fn get_all_commits_by_path(&self, path: &Path) -> Result<Vec<MetaData>, anyhow::Error> {
        let commits: Vec<commit::Model> = commit::Entity::find()
            .filter(commit::Column::RepoPath.eq(path.to_str().unwrap()))
            .all(&self.connection)
            .await
            .unwrap();
        let mut result = vec![];
        for commit in commits {
            result.push(MetaData::new(ObjectType::Commit, &commit.meta))
        }
        Ok(result)
    }

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
        let save_models: Vec<refs::ActiveModel> =
            vec![command.convert_to_model(path.to_str().unwrap())];
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
        ref_data.updated_at = Set(chrono::Utc::now().naive_utc());
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
            save_models.push(commit.convert_to_model(repo_path));
        }
        batch_save_model(conn, save_models).await.unwrap();
        Ok(true)
    }

    // async fn save_node_data(
    //     &self,
    //     save_models: Vec<node_data::ActiveModel>,
    // ) -> Result<bool, anyhow::Error> {
    //     batch_save_model(&self.connection, save_models)
    //         .await
    //         .unwrap();
    //     Ok(true)
    // }

    /// Because the requested path is a subdirectory of the original project directory,
    /// a new fake commit is needed to point the subdirectory, so we need to
    /// 1. find root commit by root_ref
    /// 2. convert commit to git Commit object,  and calculate it's hash
    /// 3. save the new fake commit with hash and repo_path
    async fn generate_child_commit_and_refs(&self, refs: &refs::Model, repo_path: &Path) -> String {
        let root_commit = commit::Entity::find()
            .filter(commit::Column::GitId.eq(&refs.ref_git_id))
            .one(&self.connection)
            .await
            .unwrap()
            .unwrap();

        if let Some(root_tree) = self.search_root_node_by_path(repo_path).await {
            let child_commit = Commit::build_from_model_and_root(&root_commit, root_tree);
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
        } else {
            ZERO_ID.to_string()
        }
    }

    async fn search_root_node_by_path(&self, repo_path: &Path) -> Option<node::Model> {
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
                // .filter(node::Column::Path.eq(repo_path.to_str().unwrap()))
                .filter(node::Column::Name.eq(""))
                .one(&self.connection)
                .await
                .unwrap()
        }
    }

    async fn get_node_by_id(&self, id: &str) -> Option<node::Model> {
        node::Entity::find()
            .filter(node::Column::GitId.eq(id))
            .one(&self.connection)
            .await
            .unwrap()
    }

    // retrieve all sub trees recursively
    #[async_recursion]
    async fn get_child_trees(&self, root: &node::Model, hash_meta: &mut HashMap<String, MetaData>) {
        let t = Tree::new(MetaData::new(ObjectType::Tree, &root.data));
        let mut child_ids = vec![];
        for item in t.tree_items {
            child_ids.push(item.id.to_plain_str());
        }
        let childs = node::Entity::find()
            .filter(node::Column::GitId.is_in(child_ids))
            .all(&self.connection)
            .await
            .unwrap();
        for c in childs {
            if c.node_type == "tree" {
                self.get_child_trees(&c, hash_meta).await;
            } else {
                let b_meta = MetaData::new(ObjectType::Blob, &c.data);
                hash_meta.insert(b_meta.id.to_plain_str(), b_meta);
            }
        }
        let t_meta = t.meta;
        tracing::info!("{}, {}", t_meta.id, t.tree_name);
        hash_meta.insert(t_meta.id.to_plain_str(), t_meta);
    }
}

// mysql sea_orm bathc insert
async fn batch_save_model<E, A>(
    conn: &DatabaseConnection,
    save_models: Vec<A>,
) -> Result<(), anyhow::Error>
where
    E: EntityTrait,
    A: ActiveModelTrait<Entity = E> + From<<E as EntityTrait>::Model> + Send,
{
    let mut futures = Vec::new();

    for chunk in save_models.chunks(200) {
        let save_result = E::insert_many(chunk.iter().cloned()).exec(conn);
        futures.push(save_result);
    }
    futures::future::join_all(futures).await;
    Ok(())
}
