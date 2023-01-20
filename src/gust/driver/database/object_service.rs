use crate::git::object::base::BaseObject;

pub trait ObjectService {
    fn search_child_objects(&self, parent: Box<dyn BaseObject>) -> Vec<Box<dyn BaseObject>>;

    fn save_objects(&self, objects: Vec<Box<dyn BaseObject>>) {}
}
