use crate::git::object::base::{
    blob::Blob,
    tree::{TreeItem, TreeItemType},
};
use std::path::PathBuf;

use crate::git::object::base::tree::Tree;

use self::nodes::{FileNode, Node, TreeNode};

use super::utils::id_generator;

pub mod nodes;

pub trait GitNodeObject {
    fn convert_to_node(&self, pid: i64) -> Box<dyn Node>;

    fn generate_id(&self) -> i64 {
        id_generator::generate_id()
    }
}

impl GitNodeObject for Tree {
    fn convert_to_node(&self, pid: i64) -> Box<dyn Node> {
        Box::new(TreeNode {
            nid: self.generate_id(),
            pid,
            oid: self.meta.id,
            name: self.tree_name.clone(),
            content_sha1: None,
            path: PathBuf::new(),
            children: Vec::new(),
        })
    }
}

impl GitNodeObject for Blob {
    fn convert_to_node(&self, pid: i64) -> Box<dyn Node> {
        Box::new(FileNode {
            nid: self.generate_id(),
            pid,
            oid: self.meta.id,
            path: PathBuf::new(),
            name: self.filename.clone(),
            content_sha1: None,
            data: Vec::new(),
        })
    }
}

impl GitNodeObject for TreeItem {
    fn convert_to_node(&self, pid: i64) -> Box<dyn Node> {
        match self.item_type {
            TreeItemType::Blob => Box::new(FileNode {
                nid: self.generate_id(),
                pid,
                oid: self.id,
                path: PathBuf::new(),
                name: self.filename.clone(),
                content_sha1: None,
                data: Vec::new(),
            }),
            TreeItemType::Tree => Box::new(TreeNode {
                nid: self.generate_id(),
                pid,
                oid: self.id,
                name: self.filename.clone(),
                content_sha1: None,
                path: PathBuf::new(),
                children: Vec::new(),
            }),
            _ => panic!("not supported type"),
        }
    }
}
