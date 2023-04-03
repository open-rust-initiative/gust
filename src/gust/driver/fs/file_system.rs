use std::{collections::HashMap, path::PathBuf};

use crate::{git::pack::Pack, gust::driver::ObjectStorage};
use async_trait::async_trait;

#[derive(Debug, Default, Clone)]
pub struct FileSystem {}

#[async_trait]
impl ObjectStorage for FileSystem {
    async fn get_head_object_id(&self, repo_dir: &PathBuf) -> String {
        let base_path = repo_dir.join(".git");
        let content = std::fs::read_to_string(base_path.join("HEAD")).unwrap();
        let content = content.replace("ref: ", "");
        let content = content.strip_suffix('\n').unwrap();
        let object_id = match std::fs::read_to_string(base_path.join(content)) {
            Ok(object_id) => object_id.strip_suffix('\n').unwrap().to_owned(),
            _ => String::from_utf8_lossy(&ZERO_ID).to_string(),
        };

        // init repo: if dir not exists or is empty
        // let init_repo = !self.repo_dir.exists();
        // todo: replace git command
        // if init_repo {
        //     Command::new("git")
        //         .args(["init", "--bare", self.repo_dir.to_str().unwrap()])
        //         .output()
        //         .expect("git init failed!");
        // }
        object_id
    }

    async fn save_packfile(
        &self,
        decoded_pack: Pack,
        repo_dir: &PathBuf,
    ) -> Result<bool, anyhow::Error> {
        todo!()
    }

    async fn get_full_pack_data(&self, repo_dir: &PathBuf) -> Vec<u8> {
        let object_root = repo_dir.join(".git/objects");
        let loose_vec = Pack::find_all_loose(object_root.to_str().unwrap());
        let (mut _loose_pack, loose_data) =
            Pack::pack_loose(loose_vec, object_root.to_str().unwrap());
        loose_data
    }

    async fn handle_pull_pack_data(&self) -> Vec<u8> {
        todo!();
    }

    async fn get_ref_object_id(&self, repo_dir: &PathBuf) -> HashMap<String, String> {
        let mut name = String::from(".git/refs/heads/");
        //TOOD: need to read from .git/packed-refs after run git gc, check how git show-ref command work
        let path = repo_dir.join(&name);
        let paths = std::fs::read_dir(&path).unwrap();
        let mut res = HashMap::new();
        for ref_file in paths.flatten() {
            name.push_str(ref_file.file_name().to_str().unwrap());
            let object_id = std::fs::read_to_string(ref_file.path()).unwrap();
            let object_id = object_id.strip_suffix('\n').unwrap();
            res.insert(object_id.to_owned(), name.to_owned());
        }
        res
    }
}
