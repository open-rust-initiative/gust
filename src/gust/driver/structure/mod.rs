use crate::git::hash::Hash;
use std::collections::HashMap;

use crate::git::{
    object::base::tree::Tree,
    pack::{decode::ObjDecodedMap, Pack},
};

use super::database::entity::node;
use super::database::mysql::mysql::GitNodeObject;

use super::ObjectStorage;

pub mod nodes;

pub async fn persist_pack_data<T: ObjectStorage>(decoded_pack: Pack, storage: &T) {
    let mut result = ObjDecodedMap::default();
    result.update_from_cache(&decoded_pack.result);
    result.check_completeness().unwrap();

    let commit = &result.commits[0];
    let tree_id = commit.tree_id;

    let ObjDecodedMap {
        commits: _,
        blobs,
        tags: _,
        trees,
        _map_hash: _,
        name_map: _,
    } = result;

    // persist_objects(commits);
    // persist_objects(tags);

    convert_and_save(&blobs, storage).await;
    convert_and_save(&trees, storage).await;

    let tree_map: HashMap<Hash, Tree> =
        trees.into_iter().map(|tree| (tree.meta.id, tree)).collect();
}

pub async fn convert_and_save<E: GitNodeObject, T: ObjectStorage>(objects: &Vec<E>, storage: &T) {
    let mut save_models: Vec<node::ActiveModel> = Vec::new();
    for obj in objects {
        save_models.push(obj.convert_to_model());
    }
    storage.persist_node_objects(save_models).await.unwrap();
}
