use sea_orm::{ActiveValue::NotSet, Set};

use crate::git::{
    hash::Hash,
    object::{
        base::{
            blob::Blob,
            commit::Commit,
            tree::{Tree, TreeItem},
        },
        metadata::MetaData,
        types::ObjectType,
    },
};
use std::{
    path::{Path, PathBuf},
    str::FromStr,
    sync::Arc,
};

use self::nodes::{FileNode, Node, TreeNode};

use super::{
    database::entity::{commit, node},
    utils::id_generator::{self, generate_id},
};

pub mod nodes;

/// only blob and tree should implement this trait
pub trait GitNodeObject {
    fn convert_to_node(&self, item: Option<&TreeItem>) -> Box<dyn Node>;

    fn convert_from_model(model: &node::Model) -> Self
    where
        Self: Sized,
    {
        todo!()
    }

    fn generate_id(&self) -> i64 {
        id_generator::generate_id()
    }
}

impl GitNodeObject for Blob {
    fn convert_to_node(&self, item: Option<&TreeItem>) -> Box<dyn Node> {
        Box::new(FileNode {
            nid: self.generate_id(),
            pid: "".to_owned(),
            git_id: self.meta.id,
            path: PathBuf::new(),
            mode: if let Some(item) = item {
                item.mode.clone()
            } else {
                Vec::new()
            },
            name: if let Some(item) = item {
                item.filename.clone()
            } else {
                "".to_owned()
            },
            data: self.meta.data.clone(),
        })
    }
    // pub fn convert_to_model(&self, node_id: i64) -> node::ActiveModel {
    //     node::ActiveModel {
    //         id: NotSet,
    //         node_id: Set(node_id),
    //         git_id: Set(self.meta.id.to_plain_str()),
    //         data: Set(self.meta.data.clone()),
    //         content_sha: NotSet,
    //         mode: Set(Vec::new()),
    //         name: Set(),
    //         node_type: Set("blob".to_owned()),
    //         created_at: Set(chrono::Utc::now().naive_utc()),
    //         updated_at: Set(chrono::Utc::now().naive_utc()),
    //     }
    // }
}

impl Commit {
    pub fn build_from_model_and_root(model: &commit::Model, root: node::Model) -> Commit {
        let mut c = Commit::new(Arc::new(MetaData::new(ObjectType::Commit, &model.meta)));
        c.tree_id = Hash::from_str(&root.git_id).unwrap();
        c.parent_tree_ids.clear();
        c.meta = Arc::new(c.encode_metadata().unwrap());
        c
    }

    pub fn convert_to_model(&self, repo_path: &Path) -> commit::ActiveModel {
        commit::ActiveModel {
            id: NotSet,
            git_id: Set(self.meta.id.to_plain_str()),
            tree: Set(self.tree_id.to_plain_str()),
            pid: NotSet,
            meta: Set(self.meta.data.clone()),
            repo_path: Set(repo_path.to_str().unwrap().to_owned()),
            author: NotSet,
            committer: NotSet,
            content: NotSet,
            created_at: Set(chrono::Utc::now().naive_utc()),
            updated_at: Set(chrono::Utc::now().naive_utc()),
        }
    }
}

impl GitNodeObject for Tree {
    // pub fn convert_from_model(model: &node::Model, tree_items: Vec<TreeItem>) -> Tree {
    //     Tree {
    //         meta: MetaData::new(ObjectType::Tree, &Vec::new()),
    //         tree_items,
    //         tree_name: model.name.clone(),
    //     }
    // }

    fn convert_to_node(&self, item: Option<&TreeItem>) -> Box<dyn Node> {
        Box::new(TreeNode {
            nid: generate_id(),
            pid: "".to_owned(),
            git_id: self.meta.id,
            name: if let Some(item) = item {
                item.filename.clone()
            } else {
                "".to_owned()
            },
            path: PathBuf::new(),
            mode: if let Some(item) = item {
                item.mode.clone()
            } else {
                Vec::new()
            },
            children: Vec::new(),
            data: self.meta.data.clone(),
        })
    }
}

// impl TreeItem {
//     pub fn convert_from_model(model: node::Model) -> TreeItem {
//         let item_type = if model.node_type == "tree" {
//             TreeItemType::Tree
//         } else {
//             TreeItemType::Blob
//         };
//         TreeItem {
//             mode: model.mode,
//             item_type,
//             id: Hash::from_bytes(model.git_id.as_bytes()).unwrap(),
//             filename: model.name,
//         }
//     }
// }

// impl GitNodeObject for TreeItem {
//     fn convert_to_node(&self) -> Box<dyn Node> {
//         match self.item_type {
//             TreeItemType::Blob => Box::new(FileNode {
//                 nid: self.generate_id(),
//                 pid: "".to_owned(),
//                 git_id: self.id,
//                 path: PathBuf::new(),
//                 mode: self.mode.clone(),
//                 name: self.filename.clone(),
//             }),
//             TreeItemType::Tree => Box::new(TreeNode {
//                 nid: self.generate_id(),
//                 pid: "".to_owned(),
//                 git_id: self.id,
//                 name: self.filename.clone(),
//                 path: PathBuf::new(),
//                 mode: self.mode.clone(),
//                 children: Vec::new(),
//             }),
//             _ => panic!("not supported type"),
//         }
//     }
// }
