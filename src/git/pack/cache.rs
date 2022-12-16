//!	Build Cache Info for the decode packed object
use std::collections::{BTreeMap, HashMap};
use crate::git::object::Metadata;

use super::super::hash::Hash;

use std::sync::Arc;

/// #### Build Cache Info for the decode packed object
/// There are two hashmap for object ,<br>
/// the keys is `hash value` of The object 
#[derive(Default,Clone)]
pub struct PackObjectCache {
  pub by_hash: BTreeMap<Hash, Arc<Metadata>>,
  pub by_offset: HashMap<Hash,u64>,
  pub offset_hash : BTreeMap<u64,Hash>,
}
// 
impl PackObjectCache{

  /// update cache by input object:`Rc<Object>` and the offset:`u64`
  pub fn update(&mut self, object: Arc<Metadata> , offset : u64 ){
    let _hash = object.id;
    self.by_hash.insert(_hash, object.clone());
    self.by_offset.insert(_hash,offset);
    self.offset_hash.insert(offset, _hash);
  }
  #[allow(unused)]
  pub fn clean(&mut self){
    self.by_hash.clear();
    self.by_offset.clear();
    self.offset_hash.clear();
  }
  

  pub fn offset_object(&mut self,offset :u64) -> Option<&mut Arc<Metadata>>{
    
    let _hash = self.offset_hash.get(&offset)?;

    self.by_hash.get_mut(_hash)

  }
  
  pub fn hash_object(&mut self,hash :Hash) -> Option<&mut Arc<Metadata>>{
    self.by_hash.get_mut(&hash)
  }
}