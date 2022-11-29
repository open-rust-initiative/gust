use std::collections::{HashMap};
use anyhow::Ok;

use super::errors::make_error;
use super::object::Object;
use super::Hash;
use std::rc::Rc;

#[derive(Default)]
pub struct PackObjectCache {
  by_hash: HashMap<Hash, Rc<Object>>,
  by_offset: HashMap<u64, Rc<Object>>,
}
impl PackObjectCache{
  pub fn update(&mut self, object: Rc<Object> , offset : u64 ){
    
    self.by_hash.insert(object.hash(), object.clone());
    self.by_offset.insert(offset, object.clone());
  }
  pub fn clean(&mut self){
    self.by_hash.clear();
    self.by_offset.clear();
   
  }
  pub fn Offset_object(&mut self,offset :u64) -> Option<&mut Rc<Object>>{
    self.by_offset.get_mut(&offset)
  }

  pub fn Hash_object(&mut self,hash :Hash) -> Option<&mut Rc<Object>>{
    self.by_hash.get_mut(&hash)
  }
}