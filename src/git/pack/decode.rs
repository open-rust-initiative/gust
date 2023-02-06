//!    Decode pack file by the `ObjDecodedMap`
//!
//!
//!
use std::collections::HashMap;
use std::fmt::{self, Display};
use std::sync::Arc;

use colored::Colorize;

use obj::base::ObjectClass;
use obj::base::{blob, commit, tag, tree};

use crate::git::errors::GitError;
use crate::git::hash::Hash;
use crate::git::object as obj;
use crate::git::object::metadata::MetaData;
use crate::git::object::types::ObjectType;
use crate::git::pack::cache::PackObjectCache;

///!对取出的object字段进行进一步解码与包装
/// 用于存储解析出的object抽象对象的hashmap
#[derive(Default, Clone)]
pub struct ObjDecodedMap {
    pub _map_hash: HashMap<Hash, Arc<ObjectClass>>,
    blobs: Vec<blob::Blob>,
    trees: Vec<tree::Tree>,
    tags: Vec<tag::Tag>,
    commits: Vec<commit::Commit>,
    name_map: HashMap<Hash, String>,
}

//在解析完object后执行的进一步的解码过程
impl ObjDecodedMap {
    /// 通过cache对不同结构进行进一步解析
    #[allow(unused)]
    pub fn update_from_cache(&mut self, cache: &PackObjectCache) {
        for (key, value) in cache.by_hash.iter() {
            let metadata = MetaData::new(value.t, &value.data);
            let _obj: ObjectClass = match value.t {
                // 交给各自的new函数,通过metadata来解码
                ObjectType::Blob => {
                    let a = blob::Blob::new(metadata);
                    self.blobs.push(a.clone());
                    ObjectClass::BLOB(a)
                }
                ObjectType::Commit => {
                    let a = commit::Commit::new(metadata);
                    self.commits.push(a.clone());
                    ObjectClass::COMMIT(a)
                }
                ObjectType::Tag => {
                    let a = tag::Tag::new(metadata);
                    self.tags.push(a.clone());
                    ObjectClass::TAG(a)
                }
                ObjectType::Tree => {
                    let a = tree::Tree::new(metadata);
                    self.trees.push(a.clone());
                    ObjectClass::TREE(a)
                }
                _ => panic!("src/git/pack/decode.rs: 33 invalid type in encoded metadata"),
            };
            self._map_hash.insert(key.clone(), Arc::new(_obj));
        }
    }

    /// 虽然这里看起来是encode的事情，但实际上还是对object的深度解析，所以放在这里了。
    /// this func should be called after the `fn update_from_cache`
    /// 这个函数做了tree种hash对象存在的校验，
    /// 对四种对象的排序 "Magic" Sort
    #[allow(unused)]
    pub fn check_completeness(&mut self) -> Result<(), GitError> {
        //验证对象树 tree object的完整性 确保tree item下的hash值有对应的object
        for _tree in self.trees.iter() {
            for item in &_tree.tree_items {
                // 保存对象名与hash值的对应
                self.name_map.insert(item.id.clone(), item.filename.clone());
                // 检查是否存在对应hash
                if self._map_hash.get(&item.id) == None {
                    return Err(GitError::UnCompletedPackObject(format!(
                        "can't find hash value:{}",
                        &_tree.meta.id
                    )));
                }
            }
        }

        // For tree & blob object , Get their name
        for _tree in self.trees.iter_mut() {
            let name = self.name_map.get(&_tree.meta.id);
            match name {
                Some(_name) => _tree.tree_name = _name.clone(),
                None => {}
            }
        }

        for _blob in self.blobs.iter_mut() {
            let name = self.name_map.get(&_blob.meta.id);
            match name {
                Some(_name) => _blob.filename = _name.clone(),
                None => {}
            }
        }
        // sort the four base object
        //TODO: This is called the "Magic" Sort
        self.trees.sort();
        self.blobs.sort();
        self.tags.sort();
        self.commits.sort();
        Ok(())
    }

    /// 将 `check_completeness` 函数解析后的放入
    #[allow(unused)]
    pub fn vec_sliding_window(&self) -> Vec<MetaData> {
        let mut list = vec![];
        for c in self.commits.iter() {
            list.push(c.meta.clone());
        }
        for t in self.tags.iter() {
            list.push(t.meta.clone());
        }
        for tree in self.trees.iter() {
            list.push(tree.meta.clone());
        }
        for blob in self.blobs.iter() {
            list.push(blob.meta.clone());
        }

        list
    }

    #[allow(unused)]
    pub fn print_vec(&self) {
        for c in self.commits.iter() {
            println!("{}", c);
        }
        for t in self.tags.iter() {
            println!("{}", t);
        }
        for tree in self.trees.iter() {
            println!("{}", tree);
        }
        for blob in self.blobs.iter() {
            println!("{}", blob);
        }
    }
}

impl Display for ObjDecodedMap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (key, value) in self._map_hash.iter() {
            writeln!(f, "*********************").unwrap();
            writeln!(f, "Hash: {}", key).unwrap();
            writeln!(f, "Type: {}", value).unwrap();
        }
        writeln!(
            f,
            "{}",
            String::from("Finish Printf for ObjDecodedMap").blue()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::super::Pack;
    use super::ObjDecodedMap;

    #[test]
    pub fn test_map_new() {
        let mut _map = ObjDecodedMap::default();
        let decoded_pack = Pack::decode_file(
            "./resources/data/test/pack-6590ba86f4e863e1c2c985b046e1d2f1a78a0089.pack",
        );
        assert_eq!(
            "6590ba86f4e863e1c2c985b046e1d2f1a78a0089",
            decoded_pack.signature.to_plain_str()
        );
        let mut result = ObjDecodedMap::default();
        result.update_from_cache(&decoded_pack.result);
        result.check_completeness().unwrap();
        result.print_vec();
    }

    // #[test]
    // fn test_object_dir_encod_temp() {
    //     let decoded_pack = Pack::decode_file(
    //         "./resources/friger/pack-6cf1ec1a89de3757f7ba776e4dc108b88367c460.pack",
    //     );
    //     println!("{}", decoded_pack.get_object_number());
    //     assert_eq!(
    //         "6cf1ec1a89de3757f7ba776e4dc108b88367c460",
    //         decoded_pack.signature.to_plain_str()
    //     );
    // }
}
