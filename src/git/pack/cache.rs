use std::collections::{HashMap};
use super::super::hash::Hash;

use super::super::object::Object;

use std::rc::Rc;

/// #### Build Cache Info for the decode packed object
/// There are two hashmap for object ,<br>
/// the keys is `hash value` and the `Offset` of The object 
#[derive(Default)]
pub struct PackObjectCache {
  pub by_hash: HashMap<Hash, Rc<Object>>,
  pub by_offset: HashMap<u64, Rc<Object>>,
}

impl PackObjectCache{

  /// update cache by input object:`Rc<Object>` and the offset:`u64`
  pub fn update(&mut self, object: Rc<Object> , offset : u64 ){
    self.by_hash.insert(object.hash(), object.clone());
    self.by_offset.insert(offset, object.clone());
  }
  #[allow(unused)]
  pub fn clean(&mut self){
    self.by_hash.clear();
    self.by_offset.clear();
  }
  
  pub fn offset_object(&mut self,offset :u64) -> Option<&mut Rc<Object>>{
    self.by_offset.get_mut(&offset)
  }
  
  pub fn hash_object(&mut self,hash :Hash) -> Option<&mut Rc<Object>>{
    self.by_hash.get_mut(&hash)
  }
}