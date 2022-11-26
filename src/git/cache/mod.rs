use std::collections::{HashMap};
use super::object::Object;
use super::Hash;
use std::rc::Rc;

#[derive(Default)]
pub struct PackObjectCache {
  by_hash: HashMap<Hash, Rc<Object>>,
  by_offset: HashMap<u64, Rc<Object>>,
}
impl PackObjectCache{
  pub fn update(&mut self, object:Object , offset :u64 ) -> Rc<Object>{
    let object_new = Rc::new(object);
    self.by_hash.insert(object.hash(), Rc::clone(&object_new));
    self.by_offset.insert(offset, Rc::clone(&object_new));
    Ok(object_new)
  }
  pub fn clean(&mut self){
    self.by_hash.clear();
    self.by_offset.clear();
  }
}