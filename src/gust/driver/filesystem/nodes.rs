use std::path::PathBuf;

use crate::{git::hash::Hash, gust::driver::StorageType};
use chrono::{DateTime, Utc};

// configuration
pub struct Repo {
    pub root: TreeNode,
    pub storage_type: StorageType,
}

pub struct Commit {
    pub id: Hash,
    pub parent_ids: Vec<Hash>,
    pub root_tree: String,
    pub msg: String,
    pub author: String,
}
// common base struct share properties wtith TreeNode and FileNode
pub struct BaseNode {
    pub id: String,
    pub object_id: Hash,
    pub content_sha1: Option<Hash>,
    pub name: String,
    pub ctime: DateTime<Utc>,
    pub mtime: Option<DateTime<Utc>>,
    pub uid: Option<String>,
    pub gid: Option<String>,
}

pub struct TreeNode {
    pub base: BaseNode,
    pub childs: Vec<Contents>,
}

pub struct Contents {
    pub id: String,
    pub name: String,
    pub file_type: String,
    pub permissions: String,
    pub size: usize,
    pub object_id: Option<Hash>,
}

pub struct FileNode {
    pub base: BaseNode,
    pub data: Option<String>,
}
/// TODO: don't know where to put this
/// the clone process will be
/// 1.parse a path from clone url
/// 2.get Node and it's children form datasource and init it
/// 3.get objects from directory structure
///
/// define the node common behaviour
pub trait NodeBehavior {
    fn init_node_from_datasource(storage_type: StorageType, path: PathBuf) -> Option<Self>
    where
        Self: Sized;

    // since we use lazy load, need manually fetch data, and might need to use a LRU cache to store the data?
    fn read_data(&self) -> String;

    // fetch all tree and blob objects from directory structure(only the current version)
    fn convert_to_objects(&self);
}
impl NodeBehavior for TreeNode {
    fn init_node_from_datasource(_storage_type: StorageType, _path: PathBuf) -> Option<TreeNode> {
        todo!()
    }

    fn read_data(&self) -> String {
        todo!()
    }

    fn convert_to_objects(&self) {
        todo!()
    }
}
impl TreeNode {
    //lazy loaded
    pub fn new(id: String, name: String, childs: Vec<Contents>) -> TreeNode {
        TreeNode {
            base: BaseNode {
                id,
                name,
                object_id: todo!(),
                content_sha1: None,
                ctime: todo!(),
                mtime: None,
                uid: None,
                gid: None,
            },
            childs,
        }
    }

    pub fn add_child(&mut self, content: Contents) {
        self.childs.push(content);
        // calculate new hash
        // self.update_hash(Hash::new())
    }

    pub fn delete_child(&self, id: String) {}

    pub fn get_child(&self, id: String) -> Option<&TreeNode> {
        // self.childs.get(&id)
        todo!()
    }

    pub fn update_child(&mut self, commit: Hash) {
        // self.node.commit = commit;
    }

    pub fn fetch_objects() {}
}

impl FileNode {
    // convert from datasource model
    pub fn new(id: String, name: String, object_id: Hash) -> FileNode {
        FileNode {
            base: BaseNode {
                id,
                name,
                object_id,
                content_sha1: None,
                ctime: todo!(),
                mtime: None,
                uid: None,
                gid: None,
            },
            data: None,
        }
    }
}

impl NodeBehavior for FileNode {
    fn init_node_from_datasource(storage_type: StorageType, path: PathBuf) -> Option<FileNode> {
        todo!()
    }

    fn read_data(&self) -> String {
        if self.data.is_none() {
            // read from datasource.
            String::new()
        } else {
            self.data.as_ref().unwrap().to_string()
        }
    }

    fn convert_to_objects(&self) {
        todo!()
    }
}

#[cfg(test)]
mod test {

    #[test]
    pub fn test1() {}
}
