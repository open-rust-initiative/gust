//!
//!
//！

use std::{collections::HashMap, path::PathBuf};

use async_trait::async_trait;

use crate::{
    gateway::api::lib::Params,
    git::{object::base::BaseObject, pack::Pack},
};

pub mod database;
pub mod fs;
pub mod structure;
pub mod utils;

#[async_trait]
pub trait ObjectStorage {
    async fn get_head_object_id(&self) -> String;

    async fn get_ref_object_id(&self) -> HashMap<String, String>;

    fn search_child_objects(
        &self,
        parent: Box<dyn BaseObject>,
    ) -> Result<Vec<Box<dyn BaseObject>>, anyhow::Error>;

    async fn save_packfile(
        &self,
        decoded_pack: Pack,
        // repo_path: &mut PathBuf,
    ) -> Result<bool, anyhow::Error>;

    async fn get_full_pack_data(&self) -> Vec<u8>;

    async fn handle_pull_pack_data(&self) -> Vec<u8>;

    fn get_path(&self) -> PathBuf;

    fn set_path(&mut self, params: Params);
}
