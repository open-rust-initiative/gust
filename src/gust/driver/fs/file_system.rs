use std::path::PathBuf;

use async_trait::async_trait;

use crate::{
    git::{object::base::BaseObject, pack::Pack, protocol::ProjectPath},
    gust::driver::ObjectStorage,
};

#[derive(Debug, Default, Clone)]
pub struct FileSystem {
    // for example: "/user/root/A/.git"
    pub work_dir: PathBuf,
}

#[async_trait]
impl ObjectStorage for FileSystem {
    async fn get_head_object_id(&self, repo_path: &str) -> String {
        let content = std::fs::read_to_string(self.work_dir.join("HEAD")).unwrap();
        let content = content.replace("ref: ", "");
        let content = content.strip_suffix('\n').unwrap();
        let object_id = match std::fs::read_to_string(self.work_dir.join(content)) {
            Ok(object_id) => object_id.strip_suffix('\n').unwrap().to_owned(),
            _ => String::from_utf8_lossy(&[b'0'; 40]).to_string(),
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

    fn search_child_objects(
        &self,
        // storage: &StorageType,
        parent: Box<dyn BaseObject>,
    ) -> Result<Vec<Box<dyn BaseObject>>, anyhow::Error> {
        todo!()
    }

    async fn save_packfile(
        &self,
        decoded_pack: Pack,
        req_path: &str,
    ) -> Result<bool, anyhow::Error> {
        todo!()
    }

    async fn get_full_pack_data(&self, path: &ProjectPath) -> Vec<u8> {
        let object_root = path.repo_dir.join(".git/objects");
        let loose_vec = Pack::find_all_loose(object_root.to_str().unwrap());
        let (mut _loose_pack, loose_data) =
            Pack::pack_loose(loose_vec, object_root.to_str().unwrap());
        loose_data
    }

    async fn handle_pull_pack_data(&self, path: &ProjectPath) -> Vec<u8> {
        todo!();
    }
}
