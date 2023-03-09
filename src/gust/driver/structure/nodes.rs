use std::{any::Any, collections::HashMap, path::PathBuf};

use crate::{
    git::{
        hash::Hash,
        object::base::tree::{Tree, TreeItemType},
        pack::{decode::ObjDecodedMap, Pack},
    },
    gust::driver::utils::id_generator::{self, generate_id},
};

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

// common base struct share properties wtith TreeNode and FileNode
// pub struct BaseNode {
//     pub id: String,
//     pub object_id: Hash,
//     pub content_sha1: Option<Hash>,
//     pub name: String,
//     pub ctime: DateTime<Utc>,
//     pub mtime: DateTime<Utc>,
// }

pub struct TreeNode {
    pub id: i64,
    pub object_id: Hash,
    pub content_sha1: Option<Hash>,
    pub name: String,
    pub path: PathBuf,
    pub children: Vec<Box<dyn Node>>,
}

// pub struct Contents {
//     pub id: String,
//     pub object_id: Hash,
//     pub content_sha1: Option<Hash>,
//     pub name: String,
//     pub file_type: String,
//     pub size: usize,
// }

#[derive(Debug, Clone)]
pub struct FileNode {
    pub id: i64,
    pub object_id: Hash,
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

    fn new(name: String) -> Self
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
}

impl Node for TreeNode {
    fn get_id(&self) -> i64 {
        self.id
    }

    fn get_name(&self) -> &str {
        &self.name
    }

    fn get_children(&self) -> &Vec<Box<dyn Node>> {
        &self.children
    }

    fn new(name: String) -> TreeNode {
        TreeNode {
            id: generate_id(),
            name,
            path: PathBuf::new(),
            object_id: Hash::default(),
            content_sha1: None,
            children: Vec::new(),
        }
    }

    fn find_child(&mut self, name: &str) -> Option<&mut Box<dyn Node>> {
        println!("{} find_child:  {}", self.name, name);
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
        self.id
    }

    fn get_name(&self) -> &str {
        &self.name
    }

    fn get_children(&self) -> &Vec<Box<dyn Node>> {
        panic!("not supported")
    }

    fn new(name: String) -> Self {
        FileNode {
            id: generate_id(),
            path: PathBuf::new(),
            name,
            object_id: Hash::default(),
            content_sha1: None,
            data: Vec::new(),
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
        id: 0,
        object_id: Hash::default(),
        content_sha1: Some(Hash::default()),
        name: "ROOT".to_owned(),
        path: PathBuf::from("/"),
        children: Vec::new(),
    };
    //TODO: load children
    Box::new(t_node)
}

pub fn persist_data(decoded_pack: Pack) {
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

    print_root(&root, 0);
}

fn build_from_root_tree(tree_id: &Hash, tree_map: &HashMap<Hash, Tree>, node: &mut Box<dyn Node>) {
    let tree = tree_map.get(tree_id).unwrap();

    for item in &tree.tree_items {
        if item.item_type == TreeItemType::Tree {
            let child_node: Box<dyn Node> = Box::new(TreeNode::new(item.filename.to_owned()));
            node.add_child(child_node);

            let child_node = match node.find_child(&item.filename) {
                Some(child) => child,
                None => panic!("Something wrong!:{}", &item.filename),
            };
            build_from_root_tree(&item.id, tree_map, child_node);
        } else {
            node.add_child(Box::new(FileNode::new(item.filename.to_owned())));
        }
    }
}

// A function to print a node with format.
pub fn print_root(node: &Box<dyn Node>, depth: u32) {
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
    if node.is_a_directory() {
        for child in node.get_children().iter() {
            print_root(child, depth + 1)
        }
    }
}

#[cfg(test)]
mod test {
    use std::path::PathBuf;

    use crate::gust::driver::{
        structure::nodes::{init_root, print_root, Node, TreeNode},
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
        print_root(&root, 0);
    }

    fn build_tree(node: &mut Box<dyn Node>, path: &PathBuf, depth: usize) {
        let parts: Vec<&str> = path.to_str().unwrap().split("/").collect();

        if depth < parts.len() {
            let child_name = parts[depth];

            let child = match node.find_child(&child_name) {
                Some(child) => child,
                None => {
                    if path.is_file() {
                        node.add_child(Box::new(FileNode::new(child_name.to_owned())));
                    } else {
                        node.add_child(Box::new(TreeNode::new(child_name.to_owned())));
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
