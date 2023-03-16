use std::{any::Any, collections::HashMap, path::PathBuf};

use sea_orm::{ActiveValue::NotSet, Set};

use crate::{
    git::{
        hash::Hash,
        object::base::{
            blob::Blob,
            tree::{Tree, TreeItemType},
        },
        pack::decode::ObjDecodedMap,
    },
    gust::driver::{
        database::entity::{node, node_data},
        utils::id_generator::{self, generate_id},
    },
};

use super::GitNodeObject;

// pub struct Repo {
//     pub root: TreeNode,
//     pub storage_type: StorageType,
//     // todo: limit the size of the cache
//     pub cache: LruCache<String, FileNode>,
// }

// pub struct Commit {
//     pub id: String,
//     pub object_id: Hash,
//     pub parent_ids: Vec<Hash>,
//     pub root_tree: Hash,
//     pub msg: String,
//     pub author: String,
// }

pub struct TreeNode {
    pub nid: i64,
    pub pid: i64,
    pub git_id: Hash,
    pub content_sha1: Option<Hash>,
    pub name: String,
    pub path: PathBuf,
    pub children: Vec<Box<dyn Node>>,
}

#[derive(Debug, Clone)]
pub struct FileNode {
    pub nid: i64,
    pub pid: i64,
    pub git_id: Hash,
    pub content_sha1: Option<Hash>,
    pub name: String,
    pub path: PathBuf,
    pub data: Vec<u8>,
}

/// the clone process will be:
/// 1. parse a path from clone url
/// 2. get Node and it's children form datasource and init it
/// 3. get objects from directory structure
/// 4. zip objects to pack and generate fake commits if necessary?
///
/// the push process might like:
/// 1. parse pack to objects, these objects are both new to the directory
/// 2. 找到这些objects对应的tree结构变动生成提交记录
///
///
/// define the node common behaviour
pub trait Node {
    fn get_id(&self) -> i64;

    fn get_git_id(&self) -> Hash;

    fn get_name(&self) -> &str;

    fn get_children(&self) -> &Vec<Box<dyn Node>>;

    fn generate_id(&self) -> i64 {
        id_generator::generate_id()
    }

    fn new(name: String, pid: i64) -> Self
    where
        Self: Sized;

    fn find_child(&mut self, name: &str) -> Option<&mut Box<dyn Node>>;

    fn add_child(&mut self, child: Box<dyn Node>);

    fn is_a_directory(&self) -> bool;

    fn as_any(&self) -> &dyn Any;

    //search in datasource by path
    fn init_node_from_datasource(path: PathBuf) -> Option<Self>
    where
        Self: Sized,
    {
        todo!()
    }

    // since we use lazy load, need manually fetch data, and might need to use a LRU cache to store the data?
    fn read_data(&self) -> String {
        "".to_string()
    }

    // fetch all tree and blob objects from directory structure(only the current version)
    fn convert_to_objects(&self) {
        todo!()
    }

    fn convert_to_model(&self) -> node::ActiveModel;
}

impl Node for TreeNode {
    fn get_id(&self) -> i64 {
        self.nid
    }
    fn get_git_id(&self) -> Hash {
        self.git_id
    }
    fn get_name(&self) -> &str {
        &self.name
    }

    fn get_children(&self) -> &Vec<Box<dyn Node>> {
        &self.children
    }

    fn new(name: String, pid: i64) -> TreeNode {
        TreeNode {
            nid: generate_id(),
            pid,
            name: name,
            path: PathBuf::new(),
            git_id: Hash::default(),
            content_sha1: None,
            children: Vec::new(),
        }
    }

    fn convert_to_model(&self) -> node::ActiveModel {
        node::ActiveModel {
            id: NotSet,
            pid: Set(self.pid),
            node_id: Set(self.nid),
            git_id: Set(self.git_id.to_plain_str()),
            node_type: Set("tree".to_owned()),
            name: Set(Some(self.name.to_string())),
            path: Set(self.path.to_str().unwrap().to_owned()),
            created_at: Set(chrono::Utc::now().naive_utc()),
            updated_at: Set(chrono::Utc::now().naive_utc()),
        }
    }

    fn find_child(&mut self, name: &str) -> Option<&mut Box<dyn Node>> {
        self.children.iter_mut().find(|c| c.get_name() == name)
    }

    fn add_child(&mut self, content: Box<dyn Node>) {
        self.children.push(content);
    }

    fn is_a_directory(&self) -> bool {
        true
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl Node for FileNode {
    fn get_id(&self) -> i64 {
        self.nid
    }
    fn get_git_id(&self) -> Hash {
        self.git_id
    }
    fn get_name(&self) -> &str {
        &self.name
    }

    fn get_children(&self) -> &Vec<Box<dyn Node>> {
        panic!("not supported")
    }

    fn new(name: String, pid: i64) -> Self {
        FileNode {
            nid: generate_id(),
            pid,
            path: PathBuf::new(),
            name,
            git_id: Hash::default(),
            content_sha1: None,
            data: Vec::new(),
        }
    }

    fn convert_to_model(&self) -> node::ActiveModel {
        node::ActiveModel {
            id: NotSet,
            pid: Set(self.pid),
            node_id: Set(self.nid),
            git_id: Set(self.git_id.to_plain_str()),
            node_type: Set("blob".to_owned()),
            name: Set(Some(self.name.to_string())),
            path: Set(self.path.to_str().unwrap().to_owned()),
            created_at: Set(chrono::Utc::now().naive_utc()),
            updated_at: Set(chrono::Utc::now().naive_utc()),
        }
    }

    fn find_child(&mut self, _name: &str) -> Option<&mut Box<dyn Node>> {
        panic!("not supported")
    }

    fn add_child(&mut self, content: Box<dyn Node>) {
        panic!("not supported")
    }

    fn is_a_directory(&self) -> bool {
        false
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

pub fn init_repo(tree: &Tree, req_path: &str) -> Box<dyn Node> {
    let t_node = TreeNode {
        nid: generate_id(),
        pid: 0,
        git_id: tree.meta.id,
        content_sha1: Some(Hash::default()),
        name: tree.tree_name.clone(),
        path: PathBuf::from(req_path),
        children: Vec::new(),
    };
    //TODO: load children
    Box::new(t_node)
}

pub struct SaveModel {
    pub nodes: Vec<node::ActiveModel>,
    pub nodes_data: Vec<node_data::ActiveModel>,
}

/// this method is used to build node tree and persist node data to database. Convert sequence:
/// 1. TreeItem => Node => Model
/// 2. Blob => Model
pub async fn build_node_tree(
    result: &ObjDecodedMap,
    req_path: &str,
) -> Result<SaveModel, anyhow::Error> {
    let commit = &result.commits[0];
    let tree_id = commit.tree_id;
    let tree_map: HashMap<Hash, Tree> = result
        .trees
        .clone()
        .into_iter()
        .map(|tree| (tree.meta.id, tree))
        .collect();
    let blob_map: HashMap<Hash, Blob> = result
        .blobs
        .clone()
        .into_iter()
        .map(|b| (b.meta.id, b))
        .collect();

    let mut root = init_repo(tree_map.get(&tree_id).unwrap(), req_path);
    build_from_root_tree(&tree_id, &tree_map, &mut root, req_path);
    let mut save_model = SaveModel {
        nodes: Vec::new(),
        nodes_data: Vec::new(),
    };
    traverse_node(root.as_ref(), 0, &mut save_model, &blob_map);
    Ok(save_model)
}

/// convert TreeItem to Node and build node tree
pub fn build_from_root_tree(
    tree_id: &Hash,
    tree_map: &HashMap<Hash, Tree>,
    node: &mut Box<dyn Node>,
    req_path: &str,
) {
    let tree = tree_map.get(tree_id).unwrap();

    for item in &tree.tree_items {
        if item.item_type == TreeItemType::Tree {
            let child_node: Box<dyn Node> = item.convert_to_node(node.get_id(), req_path);
            node.add_child(child_node);

            let child_node = match node.find_child(&item.filename) {
                Some(child) => child,
                None => panic!("Something wrong!:{}", &item.filename),
            };
            build_from_root_tree(&item.id, tree_map, child_node, req_path);
        } else {
            node.add_child(item.convert_to_node(node.get_id(), req_path));
        }
    }
}

/// conver Node to db entity and for later persistent
pub fn traverse_node(
    node: &dyn Node,
    depth: u32,
    save_model: &mut SaveModel,
    blob_map: &HashMap<Hash, Blob>,
) {
    print_node(node, depth);
    let SaveModel { nodes, nodes_data } = save_model;
    nodes.push(node.convert_to_model());
    if node.is_a_directory() {
        for child in node.get_children().iter() {
            traverse_node(child.as_ref(), depth + 1, save_model, blob_map);
        }
    } else {
        nodes_data.push(
            blob_map
                .get(&node.get_git_id())
                .unwrap()
                .convert_to_model(node.get_id()),
        );
    }
}

/// Print a node with format.
pub fn print_node(node: &dyn Node, depth: u32) {
    if depth == 0 {
        println!("{}", node.get_name());
    } else {
        println!(
            "{:indent$}└── {} {}",
            "",
            node.get_name(),
            node.get_id(),
            indent = ((depth as usize) - 1) * 4
        );
    }
}

#[cfg(test)]
mod test {
    use std::path::PathBuf;

    use crate::gust::driver::{
        database::entity::node,
        structure::nodes::{traverse_node, Node, TreeNode},
        utils::id_generator,
    };

    use super::FileNode;

    #[test]
    pub fn main() {
        // Form our INPUT:  a list of paths.
        let paths = vec![
            PathBuf::from("child1/grandchild1.txt"),
            PathBuf::from("child1/grandchild2.txt"),
            PathBuf::from("child2/grandchild3.txt"),
            PathBuf::from("child3"),
        ];
        println!("Input Paths:\n{:#?}\n", paths);
        id_generator::set_up_options().unwrap();
        // let mut root = init_root();
        // for path in paths.iter() {
        //     build_tree(&mut root, path, 0)
        // }

        // let mut save_models: Vec<node::ActiveModel> = Vec::new();

        // traverse_node(root.as_ref(), 0, &mut save_models);
    }

    fn build_tree(node: &mut Box<dyn Node>, path: &PathBuf, depth: usize) {
        let parts: Vec<&str> = path.to_str().unwrap().split("/").collect();

        if depth < parts.len() {
            let child_name = parts[depth];

            let child = match node.find_child(&child_name) {
                Some(child) => child,
                None => {
                    if path.is_file() {
                        node.add_child(Box::new(FileNode::new(child_name.to_owned(), 0)));
                    } else {
                        node.add_child(Box::new(TreeNode::new(child_name.to_owned(), 0)));
                    };
                    match node.find_child(&child_name) {
                        Some(child) => child,
                        None => panic!("Something wrong!:{}, {}", &child_name, depth),
                    }
                }
            };
            build_tree(child, path, depth + 1);
        }
    }
}
