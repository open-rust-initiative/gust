//!
//!
//!
//!
//!
//!
//!

use crate::errors::GitError;
use crate::git::Metadata;
use crate::git::tree::{TreeItem, TreeItemType};

/// Git Object: blob
#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone)]
pub struct Blob {
    pub meta: Metadata,
    pub data: Vec<u8>,
}

///
impl Blob {
    ///
    #[allow(unused)]
    pub(crate) fn write_to_file(&self, root_path: String) -> Result<String, GitError> {
        self.meta.write_to_file(root_path)
    }

    ///
    #[allow(unused)]
    pub(crate) fn to_tree_item(&self, filename: String) -> Result<TreeItem, ()> {
        Ok(
            TreeItem {
                mode: TreeItemType::Blob.to_bytes().to_vec(),
                item_type: TreeItemType::Blob,
                id: self.meta.id.clone(),
                filename,
            }
        )

    }
}

///
#[cfg(test)]
mod tests {
    use std::env;
    use std::fs::File;
    use std::io::BufReader;
    use std::io::Read;
    use std::path::{Path, PathBuf};

    use bstr::ByteSlice;

    use crate::git::id::ID;
    use crate::git::Type;
    use crate::git::hash::Hash;
    use crate::git::Metadata;

    use super::Blob;
    ///
    #[test]
    fn test_blob_write_to_file() {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("resources/data/test/gitmega.md");
        let f = File::open(path).ok();
        let mut reader = BufReader::new(f.unwrap());
        let mut buffer = Vec::new();
        reader.read_to_end(&mut buffer).ok();

        if env::consts::OS == "windows" {
            buffer = buffer.replace(b"\r\n", b"\n");
        }

        let id = ID::from_vec(Type::Blob, &mut buffer);
        let size = buffer.len();
        let data = buffer;

        let meta = crate::git::Metadata {
            t: Type::Blob,
            h: Hash::Sha1,
            id,
            size,
            data,
        };

        meta.write_to_file("/tmp".to_string())
            .expect("Write error!");
        assert!(Path::new("/tmp/82/352c3a6a7a8bd32011751699c7a3648d1b5d3c").exists());
    }

    ///
    #[test]
    fn test_blob_read_from_file() {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("resources/data/test/blob-82352c3a6a7a8bd32011751699c7a3648d1b5d3c-gitmega.md");

        let meta =
            Metadata::read_object_from_file(path.to_str().unwrap().to_string())
                .expect("Read error!");

        assert_eq!(meta.t, crate::git::Type::Blob);

        let blob = Blob {
            meta: meta.clone(),
            data: meta.data,
        };

        assert_eq!(
            "82352c3a6a7a8bd32011751699c7a3648d1b5d3c",
            blob.meta.id.to_string()
        );
        assert_eq!(16, blob.meta.size);
        assert_eq!(
            "# Hello Gitmega\n",
            String::from_utf8(blob.data).unwrap().as_str()
        );
    }
}