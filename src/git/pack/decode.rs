use std::{collections::HashMap, rc::Rc};
use crate::git::Metadata;
use super::ID;
use crate::git::hash::{Hash,HashType};

use crate::git::object::types::ObjectType;

use crate::git::ObjClass;
use super::super::{blob,commit,tag,tree};
use super::cache::PackObjectCache;
///!对取出的object字段进行进一步解码与包装
/// 

#[derive(Default)]
pub struct ObjDecodedMap{
   pub _map_hash:HashMap<Hash,Rc<ObjClass>>
}//
//在解析完object后执行的进一步的解码过程
impl ObjDecodedMap {
    #[allow(unused)]
    pub fn update_from_cache(&mut self, cache:& PackObjectCache) {
        for (key, value) in cache.by_hash.iter() {
            let metadata = 
                Metadata {
                    t: value.object_type ,  
                    h: HashType::Sha1,
                    id: ID::from_bytes(&value.hash().0),
                    size: value.contents.len(),
                    data:value.contents.to_vec(),
                };
            
            let _obj:ObjClass=match value.object_type {// 交给各自的new函数,通过metadata来解码
                ObjectType::Blob => ObjClass::BLOB(blob::Blob::new(metadata)),
                ObjectType::Commit => ObjClass::COMMIT(commit::Commit::new(metadata) ),
                ObjectType::Tag => ObjClass::TAG(tag::Tag::new(metadata)),
                ObjectType::Tree =>  ObjClass::TREE(tree::Tree::new(metadata)),
            }; 
            self._map_hash.insert(key.clone(),Rc::new(_obj));
        }
        
    }
}


#[cfg(test)]
mod tests {
    
    #[test]
    pub fn test_map_new(){
//TODO 写map的测试
    }
}