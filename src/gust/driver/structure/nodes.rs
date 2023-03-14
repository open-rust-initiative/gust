use std::{any::Any, collections::HashMap, path::PathBuf};

use sea_orm::{ActiveValue::NotSet, Set};

use crate::{
    git::{
        hash::Hash,
        object::base::tree::{Tree, TreeItemType},
        pack::{decode::ObjDecodedMap, Pack},
    },
    gust::driver::{
        database::entity::node,
        utils::id_generator::{self, generate_id},
        ObjectStorage,
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
    pub oid: Hash,
    pub content_sha1: Option<Hash>,
    pub name: String,
    pub path: PathBuf,
    pub children: Vec<Box<dyn Node>>,
}

#[derive(Debug, Clone)]
pub struct FileNode {
    pub nid: i64,
    pub pid: i64,
    pub oid: Hash,
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
/// 在内存中维护Tree目录结构
/// 如何初始化？
///
/// define the node common behaviour
pub trait Node {
    fn get_id(&self) -> i64;

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
            oid: Hash::default(),
            content_sha1: None,
            children: Vec::new(),
        }
    }

    fn convert_to_model(&self) -> node::ActiveModel {
        node::ActiveModel {
            id: NotSet,
            pid: Set(self.pid),
            node_id: Set(self.nid),
            oid: Set(self.oid.to_plain_str()),
            node_type: Set("tree".to_owned()),
            content_sha1: NotSet,
            name: Set(Some(self.name.to_string())),
            path: NotSet,
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
            oid: Hash::default(),
            content_sha1: None,
            data: Vec::new(),
        }
    }

    fn convert_to_model(&self) -> node::ActiveModel {
        node::ActiveModel {
            id: NotSet,
            pid: Set(self.pid),
            node_id: Set(self.nid),
            oid: Set(self.oid.to_plain_str()),
            node_type: Set("blob".to_owned()),
            content_sha1: NotSet,
            name: Set(Some(self.name.to_string())),
            path: NotSet,
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

pub fn init_root() -> Box<dyn Node> {
    let t_node = TreeNode {
        nid: 0,
        pid: 0,
        oid: Hash::default(),
        content_sha1: Some(Hash::default()),
        name: "ROOT".to_owned(),
        path: PathBuf::from("/"),
        children: Vec::new(),
    };
    //TODO: load children
    Box::new(t_node)
}

/// this method is used to persist tree and blob objects to database
/// 解析成tree，
/// 检查数据库是否存在该数据，不存在save
///
pub fn build_from_pack(decoded_pack: Pack) -> Vec<node::ActiveModel> {
    let mut result = ObjDecodedMap::default();
    result.update_from_cache(&decoded_pack.result);
    result.check_completeness().unwrap();

    let commit = &result.commits[0];
    let tree_id = commit.tree_id;
    let tree_map: HashMap<Hash, Tree> = result
        .trees
        .into_iter()
        .map(|tree| (tree.meta.id, tree))
        .collect();
    let mut root = init_root();

    build_from_root_tree(&tree_id, &tree_map, &mut root);

    let mut save_models: Vec<node::ActiveModel> = Vec::new();
    traverse_node(root.as_ref(), 0, &mut save_models);
    save_models
}

/// convert TreeItem to Node and build node tree
fn build_from_root_tree(tree_id: &Hash, tree_map: &HashMap<Hash, Tree>, node: &mut Box<dyn Node>) {
    let tree = tree_map.get(tree_id).unwrap();

    for item in &tree.tree_items {
        if item.item_type == TreeItemType::Tree {
            let child_node: Box<dyn Node> = item.convert_to_node(node.get_id());
            node.add_child(child_node);

            let child_node = match node.find_child(&item.filename) {
                Some(child) => child,
                None => panic!("Something wrong!:{}", &item.filename),
            };
            build_from_root_tree(&item.id, tree_map, child_node);
        } else {
            node.add_child(item.convert_to_node(node.get_id()));
        }
    }
}

/// conver Node to db entity and for later persistent
pub fn traverse_node(node: &dyn Node, depth: u32, model_list: &mut Vec<node::ActiveModel>) {
    print_node(node, depth);
    model_list.push(node.convert_to_model());
    if node.is_a_directory() {
        for child in node.get_children().iter() {
            traverse_node(child.as_ref(), depth + 1, model_list);
        }
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
        structure::nodes::{init_root, traverse_node, Node, TreeNode},
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
        let mut root = init_root();
        for path in paths.iter() {
            build_tree(&mut root, path, 0)
        }

        let mut save_models: Vec<node::ActiveModel> = Vec::new();

        traverse_node(root.as_ref(), 0, &mut save_models);
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
