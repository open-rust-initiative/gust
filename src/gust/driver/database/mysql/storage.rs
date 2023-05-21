use std::cmp::min;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::str::FromStr;
use std::sync::Arc;

use crate::git::errors::{GitError, GitLFSError};
use crate::git::hash::Hash;
use crate::git::lfs::structs::*;
use crate::git::object::base::commit::Commit;
use crate::git::object::base::tree::Tree;
use crate::git::object::metadata::MetaData;
use crate::git::object::types::ObjectType;
use crate::git::pack::decode::ObjDecodedMap;
use crate::git::pack::Pack;
use crate::git::protocol::{Command, RefCommand};
use crate::gust::driver::structure::nodes::build_node_tree;
use crate::gust::driver::{ObjectStorage, ZERO_ID};
use async_recursion::async_recursion;
use async_trait::async_trait;
use chrono::prelude::*;
use entity::{commit, locks, meta, node, refs};
use futures::lock;
use rayon::vec;
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
        let nodes = build_node_tree(&result, repo_path).await.unwrap();
        self.save_nodes(nodes).await.unwrap();
        self.save_commits(&result.commits, repo_path).await.unwrap();
        Ok(())
    }

    async fn get_full_pack_data(&self, repo_path: &Path) -> Result<Vec<u8>, GitError> {
        let mut hash_meta: HashMap<String, MetaData> = HashMap::new();

        let commit_metas = self.get_all_commits_by_path(repo_path).await.unwrap();
        let mut commits = Vec::new();
        let mut tree_ids = Vec::new();

        for c_meta in commit_metas {
            let c = Commit::new(Arc::new(c_meta));
            tree_ids.push(c.tree_id.to_plain_str());
            commits.push(c);
        }
        let trees = self.get_nodes_by_ids(tree_ids).await;
        for commit in commits {
            hash_meta.insert(
                commit.meta.id.to_plain_str(),
                Arc::try_unwrap(commit.meta).unwrap(),
            );
            if let Some(root) = trees.get(&commit.tree_id) {
                self.get_child_trees(&root, &mut hash_meta).await
            } else {
                return Err(GitError::InvalidTreeObject(commit.tree_id.to_plain_str()));
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
                let c = Commit::new(Arc::new(c_meta));
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

    async fn lfs_get_meta(&self, v: &RequestVars) -> Result<MetaObject, GitLFSError> {
        let result = meta::Entity::find_by_id(v.oid.clone())
            .one(&self.connection)
            .await
            .unwrap();

        match result {
            Some(val) => Ok(MetaObject {
                oid: val.oid,
                size: val.size,
                exist: val.exist,
            }),
            None => Err(GitLFSError::GeneralError("".to_string())),
        }
    }

    async fn lfs_put_meta(&self, v: &RequestVars) -> Result<MetaObject, GitLFSError> {
        // Check if already exist.
        let result = meta::Entity::find_by_id(v.oid.clone())
            .one(&self.connection)
            .await
            .unwrap();
        if result.is_some() {
            let result = result.unwrap();
            return Ok(MetaObject {
                oid: result.oid,
                size: result.size,
                exist: true,
            });
        }

        // Put into database if not exist.
        let meta = MetaObject {
            oid: v.oid.to_string(),
            size: v.size,
            exist: true,
        };

        let meta_to = meta::ActiveModel {
            oid: Set(meta.oid.to_owned()),
            size: Set(meta.size.to_owned()),
            exist: Set(true),
        };

        let res = meta::Entity::insert(meta_to).exec(&self.connection).await;
        match res {
            Ok(_) => Ok(meta),
            Err(_) => Err(GitLFSError::GeneralError("".to_string())),
        }
    }

    async fn lfs_delete_meta(&self, v: &RequestVars) -> Result<(), GitLFSError> {
        let res = meta::Entity::delete_by_id(v.oid.to_owned())
            .exec(&self.connection)
            .await;
        match res {
            Ok(_) => Ok(()),
            Err(_) => Err(GitLFSError::GeneralError("".to_string())),
        }
    }

    async fn lfs_get_locks(&self, refspec: &str) -> Result<Vec<Lock>, GitLFSError> {
        let result = locks::Entity::find_by_id(refspec)
            .one(&self.connection)
            .await
            .unwrap();

        match result {
            Some(val) => {
                let data = val.data.to_owned();
                let locks: Vec<Lock> = serde_json::from_str(&data).unwrap();
                Ok(locks)
            }
            None => Err(GitLFSError::GeneralError("".to_string())),
        }
    }

    async fn lfs_get_filtered_locks(
        &self,
        refspec: &str,
        path: &str,
        cursor: &str,
        limit: &str,
    ) -> Result<(Vec<Lock>, String), GitLFSError> {
        let mut locks = match self.lfs_get_locks(refspec).await {
            Ok(locks) => locks,
            Err(_) => vec![],
        };

        println!("Locks retrieved: {:?}", locks);

        if cursor != "" {
            let mut last_seen = -1;
            for (i, v) in locks.iter().enumerate() {
                if v.id == *cursor {
                    last_seen = i as i32;
                    break;
                }
            }

            if last_seen > -1 {
                locks = locks.split_off(last_seen as usize);
            } else {
                // Cursor not found.
                return Err(GitLFSError::GeneralError("".to_string()));
            }
        }

        if path != "" {
            let mut filterd = Vec::<Lock>::new();
            for lock in locks.iter() {
                if lock.path == *path {
                    filterd.push(Lock {
                        id: lock.id.to_owned(),
                        path: lock.path.to_owned(),
                        owner: lock.owner.clone(),
                        locked_at: lock.locked_at.to_owned(),
                    });
                }
            }
            locks = filterd;
        }

        let mut next = "".to_string();
        if limit != "" {
            let mut size = limit.parse::<i64>().unwrap();
            size = min(size, locks.len() as i64);

            if size + 1 < locks.len() as i64 {
                next = locks[size as usize].id.to_owned();
            }
            let _ = locks.split_off(size as usize);
        }

        Ok((locks, next))
    }

    async fn lfs_add_lock(&self, repo: &str, locks: Vec<Lock>) -> Result<(), GitLFSError> {
        let result = locks::Entity::find_by_id(repo.to_owned())
            .one(&self.connection)
            .await
            .unwrap();

        match result {
            // Update
            Some(val) => {
                let d = val.data.to_owned();
                let mut locks_from_data = if d != "" {
                    let locks_from_data: Vec<Lock> = serde_json::from_str(&d).unwrap();
                    locks_from_data
                } else {
                    vec![]
                };
                let mut locks = locks;
                locks_from_data.append(&mut locks);

                locks_from_data.sort_by(|a, b| {
                    a.locked_at
                        .partial_cmp(&b.locked_at)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
                let d = serde_json::to_string(&locks_from_data).unwrap();

                let mut lock_to: locks::ActiveModel = val.into();
                lock_to.data = Set(d.to_owned());
                let res = lock_to.update(&self.connection).await;
                match res.is_ok() {
                    true => Ok(()),
                    false => Err(GitLFSError::GeneralError("".to_string())),
                }
            }
            // Insert
            None => {
                let mut locks = locks;
                locks.sort_by(|a, b| {
                    a.locked_at
                        .partial_cmp(&b.locked_at)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
                let data = serde_json::to_string(&locks).unwrap();
                let lock_to = locks::ActiveModel {
                    id: Set(repo.to_owned()),
                    data: Set(data.to_owned()),
                };
                let res = locks::Entity::insert(lock_to).exec(&self.connection).await;
                match res.is_ok() {
                    true => Ok(()),
                    false => Err(GitLFSError::GeneralError("".to_string())),
                }
            }
        }
    }

    async fn lfs_delete_lock(
        &self,
        repo: &str,
        _user: Option<String>,
        id: &str,
        force: bool,
    ) -> Result<Lock, GitLFSError> {
        let empty_lock = Lock {
            id: "".to_owned(),
            path: "".to_owned(),
            owner: None,
            locked_at: {
                let locked_at: DateTime<Utc> = DateTime::<Utc>::MIN_UTC;
                locked_at.to_rfc3339().to_string()
            },
        };
        let result = locks::Entity::find_by_id(repo.to_owned())
            .one(&self.connection)
            .await
            .unwrap();

        match result {
            // Exist, then delete.
            Some(val) => {
                let d = val.data.to_owned();
                let locks_from_data = if d != "" {
                    let locks_from_data: Vec<Lock> = serde_json::from_str(&d).unwrap();
                    locks_from_data
                } else {
                    vec![]
                };

                let mut new_locks = Vec::<Lock>::new();
                let mut lock_to_delete = Lock {
                    id: "".to_owned(),
                    path: "".to_owned(),
                    owner: None,
                    locked_at: {
                        let locked_at: DateTime<Utc> = DateTime::<Utc>::MIN_UTC;
                        locked_at.to_rfc3339().to_string()
                    },
                };

                for lock in locks_from_data.iter() {
                    if lock.id == *id {
                        if lock.owner != None && !force {
                            return Err(GitLFSError::GeneralError("".to_string()));
                        }
                        lock_to_delete.id = lock.id.to_owned();
                        lock_to_delete.path = lock.path.to_owned();
                        lock_to_delete.owner = lock.owner.clone();
                        lock_to_delete.locked_at = lock.locked_at.to_owned();
                    } else if lock.id.len() > 0 {
                        new_locks.push(Lock {
                            id: lock.id.to_owned(),
                            path: lock.path.to_owned(),
                            owner: lock.owner.clone(),
                            locked_at: lock.locked_at.to_owned(),
                        });
                    }
                }
                if lock_to_delete.id == "" {
                    return Err(GitLFSError::GeneralError("".to_string()));
                }

                // No locks remains, delete the repo from database.
                if new_locks.len() == 0 {
                    locks::Entity::delete_by_id(repo.to_owned())
                        .exec(&self.connection)
                        .await
                        .unwrap();

                    return Ok(lock_to_delete);
                }

                // Update remaining locks.
                let data = serde_json::to_string(&new_locks).unwrap();

                let mut lock_to: locks::ActiveModel = val.into();
                lock_to.data = Set(data.to_owned());
                let res = lock_to.update(&self.connection).await;
                match res.is_ok() {
                    true => Ok(lock_to_delete),
                    false => Err(GitLFSError::GeneralError("".to_string())),
                }
            }
            // Not exist, error.
            None => Err(GitLFSError::GeneralError("".to_string())),
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

    async fn save_nodes(&self, nodes: Vec<node::ActiveModel>) -> Result<bool, anyhow::Error> {
        let conn = &self.connection;
        let mut sum = 0;
        let mut batch_nodes = Vec::new();
        for node in nodes {
            // let model = node.try_into_model().unwrap();
            let size = node.data.as_ref().len();
            let limit = 10 * 1024 * 1024;
            if sum + size < limit && batch_nodes.len() < 50 {
                sum += size;
                batch_nodes.push(node);
            } else {
                node::Entity::insert_many(batch_nodes)
                    .exec(conn)
                    .await
                    .unwrap();
                sum = 0;
                batch_nodes = Vec::new();
                batch_nodes.push(node);
            }
        }
        if batch_nodes.len() != 0 {
            node::Entity::insert_many(batch_nodes)
                .exec(conn)
                .await
                .unwrap();
        }
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

    async fn get_nodes_by_ids(&self, ids: Vec<String>) -> HashMap<Hash, node::Model> {
        node::Entity::find()
            .filter(node::Column::GitId.is_in(ids))
            .all(&self.connection)
            .await
            .unwrap()
            .into_iter()
            .map(|f| (Hash::from_str(&f.git_id).unwrap(), f))
            .collect()
    }

    // retrieve all sub trees recursively
    #[async_recursion]
    async fn get_child_trees(&self, root: &node::Model, hash_meta: &mut HashMap<String, MetaData>) {
        let t = Tree::new(Arc::new(MetaData::new(ObjectType::Tree, &root.data)));
        let mut child_ids = vec![];
        for item in t.tree_items {
            if !hash_meta.contains_key(&item.id.to_plain_str()) {
                child_ids.push(item.id.to_plain_str());
            }
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
        hash_meta.insert(t_meta.id.to_plain_str(), Arc::try_unwrap(t_meta).unwrap());
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

    // notice that sqlx not support packets larger than 16MB now
    for chunk in save_models.chunks(100) {
        let save_result = E::insert_many(chunk.iter().cloned()).exec(conn).await;
        futures.push(save_result);
    }
    // futures::future::join_all(futures).await;
    Ok(())
}
