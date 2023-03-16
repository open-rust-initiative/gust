use sea_orm::{ActiveValue::NotSet, Set};

use crate::git::object::base::{
    blob::Blob,
    commit::Commit,
    tree::{TreeItem, TreeItemType},
};
use std::path::PathBuf;

use crate::git::object::base::tree::Tree;

use self::nodes::{FileNode, Node, TreeNode};

use super::{
    database::entity::{commit, node_data},
    utils::id_generator,
};

pub mod nodes;

pub trait GitNodeObject {
    fn convert_to_node(&self, pid: i64, req_path: &str) -> Box<dyn Node>;

    fn generate_id(&self) -> i64 {
        id_generator::generate_id()
    }
}

impl GitNodeObject for Tree {
    fn convert_to_node(&self, pid: i64, req_path: &str) -> Box<dyn Node> {
        Box::new(TreeNode {
            nid: self.generate_id(),
            pid,
            git_id: self.meta.id,
            name: self.tree_name.clone(),
            content_sha1: None,
            path: PathBuf::from(req_path),
            children: Vec::new(),
        })
    }
}

impl GitNodeObject for Blob {
    fn convert_to_node(&self, pid: i64, req_path: &str) -> Box<dyn Node> {
        Box::new(FileNode {
            nid: self.generate_id(),
            pid,
            git_id: self.meta.id,
            path: PathBuf::from(req_path),
            name: self.filename.clone(),
            content_sha1: None,
            data: Vec::new(),
        })
    }
}

impl Blob {
    pub fn convert_to_model(&self, node_id: i64) -> node_data::ActiveModel {
        node_data::ActiveModel {
            id: NotSet,
            node_id: Set(node_id),
            git_id: Set(self.meta.id.to_plain_str()),
            data: Set(self.meta.data.clone()),
            content_sha1: NotSet,
        }
    }
}

impl Commit {
    pub fn convert_to_model(&self, repo_path: &str, meta: &[u8]) -> commit::ActiveModel {
        commit::ActiveModel {
            id: NotSet,
            git_id: Set(self.meta.id.to_plain_str()),
            tree: Set(self.tree_id.to_plain_str()),
            pid: NotSet,
            meta: Set(meta.to_vec()),
            // is_head: Set(false),
            repo_path: Set(repo_path.to_string()),
            author: NotSet,
            committer: NotSet,
            content: NotSet,
            created_at: Set(chrono::Utc::now().naive_utc()),
            updated_at: Set(chrono::Utc::now().naive_utc()),
        }
    }
}

impl GitNodeObject for TreeItem {
    fn convert_to_node(&self, pid: i64, req_path: &str) -> Box<dyn Node> {
        match self.item_type {
            TreeItemType::Blob => Box::new(FileNode {
                nid: self.generate_id(),
                pid,
                git_id: self.id,
                path: PathBuf::from(req_path),
                name: self.filename.clone(),
                content_sha1: None,
                data: Vec::new(),
            }),
            TreeItemType::Tree => Box::new(TreeNode {
                nid: self.generate_id(),
                pid,
                git_id: self.id,
                name: self.filename.clone(),
                content_sha1: None,
                path: PathBuf::from(req_path),
                children: Vec::new(),
            }),
            _ => panic!("not supported type"),
        }
    }
}
