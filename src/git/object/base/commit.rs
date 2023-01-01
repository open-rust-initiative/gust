//!
//!Commit 对象结构体
//!

use super::super::Hash;
use super::sign::AuthorSign;
use super::Metadata;
use crate::errors::GustError;
use crate::git::object::types::ObjectType;
use bstr::ByteSlice;
use std::cmp::Ordering;
use std::fmt::Display;
use crate::git::errors::GitError;

/// Git Object: commit
#[allow(unused)]
#[derive(Eq, Debug, Hash, Clone)]
pub struct Commit {
    pub meta: Metadata,
    pub tree_id: Hash,
    pub parent_tree_ids: Vec<Hash>,
    pub author: AuthorSign,
    pub committer: AuthorSign,
    pub message: String,
}
impl Ord for Commit {
    fn cmp(&self, other: &Self) -> Ordering {
        other.meta.size.cmp(&self.meta.size)
    }
}

impl PartialOrd for Commit {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(other.meta.size.cmp(&self.meta.size))
    }
}

impl PartialEq for Commit {
    fn eq(&self, other: &Self) -> bool {
        self.meta.size == other.meta.size
    }
}
///
impl Commit {
    ///
    pub fn new(metadata: Metadata) -> Self {
        let mut a = Self {
            meta: metadata,
            tree_id: Hash::default(),
            parent_tree_ids: vec![],
            author: AuthorSign {
                t: "".to_string(),
                name: "".to_string(),
                email: "".to_string(),
                timestamp: 0,
                timezone: "".to_string(),
            },
            committer: AuthorSign {
                t: "".to_string(),
                name: "".to_string(),
                email: "".to_string(),
                timestamp: 0,
                timezone: "".to_string(),
            },
            message: "".to_string(),
        };
        a.decode_meta().unwrap();
        a
    }
    ///

    /// Decode the Metadata.data and convert to `Commit` Class
    // If there a
    pub(crate) fn decode_meta(&mut self) -> Result<(), GustError> {
        let mut data = self.meta.data.clone();

        // Find the tree id and remove it from the data
        let tree_begin = data.find_byte(0x20).unwrap();
        let tree_end = data.find_byte(0x0a).unwrap();
        self.tree_id = Hash::from_bytes(&data[tree_begin + 1..tree_end].to_vec()).unwrap();
        data = data[tree_end + 1..].to_vec();

        // Find the parent tree ids and remove them from the data
        let author_begin = data.find("author").unwrap();
        if data.find_iter("parent").count() > 0 {
            let mut parents: Vec<Hash> = Vec::new();
            let mut index = 0;

            while index < author_begin {
                let parent_begin = data.find_byte(0x20).unwrap();
                let parent_end = data.find_byte(0x0a).unwrap();
                parents
                    .push(Hash::from_bytes(&data[parent_begin + 1..parent_end].to_vec()).unwrap());
                index = index + parent_end + 1;
            }

            self.parent_tree_ids = parents;
        }
        data = data[author_begin..].to_vec();

        // Find the author and remove it from the data
        let author_data = data[..data.find_byte(0x0a).unwrap()].to_vec();
        self.author.decode_from_data(author_data)?;
        data = data[data.find_byte(0x0a).unwrap() + 1..].to_vec();

        // Find the committer and remove it from the data
        let committer_data = data[..data.find_byte(0x0a).unwrap()].to_vec();
        self.committer.decode_from_data(committer_data)?;
        self.message = data[data.find_byte(0x0a).unwrap() + 1..]
            .to_vec()
            .to_str()
            .unwrap()
            .to_string();

        Ok(())
    }

    ///
    #[allow(unused)]
    pub(crate) fn write_to_file(&self, root_path: String) -> Result<String, GitError> {
        self.meta.write_to_file(root_path)
    }

    ///
    #[allow(unused)]
    pub(crate) fn encode_metadata(&self) -> Result<Metadata, ()> {
        let mut data = Vec::new();

        data.extend_from_slice("tree".as_bytes());
        data.extend_from_slice(0x20u8.to_be_bytes().as_ref());
        data.extend_from_slice(self.tree_id.to_plain_str().as_bytes());
        data.extend_from_slice(0x0au8.to_be_bytes().as_ref());

        for parent_tree_id in &self.parent_tree_ids {
            data.extend_from_slice("parent".as_bytes());
            data.extend_from_slice(0x20u8.to_be_bytes().as_ref());
            data.extend_from_slice(parent_tree_id.to_plain_str().as_bytes());
            data.extend_from_slice(0x0au8.to_be_bytes().as_ref());
        }

        data.extend_from_slice(self.author.encode_to_data().unwrap().as_ref());
        data.extend_from_slice(0x0au8.to_be_bytes().as_ref());
        data.extend_from_slice(self.committer.encode_to_data().unwrap().as_ref());
        data.extend_from_slice(0x0au8.to_be_bytes().as_ref());
        data.extend_from_slice(self.message.as_bytes());

        Ok(Metadata::new(ObjectType::Commit, &data))
    }
}

impl Display for Commit {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        writeln!(f, "Tree: {}", self.tree_id)?;

        for parent in self.parent_tree_ids.iter() {
            writeln!(f, "parent: {}", parent)?;
        }

        writeln!(f, "author {}", self.author)?;
        writeln!(f, "committer {}", self.committer)?;
        writeln!(f, "Message: {}", self.message)
    }
}

#[cfg(test)]
mod tests {
    use super::AuthorSign;
    use super::Metadata;
    use crate::git::hash::Hash;
    use crate::git::object::types::ObjectType;
    use std::env;
    use std::path::Path;
    use std::path::PathBuf;
    use std::str::FromStr;

    use super::Commit;

    fn get_empty_commit(path: PathBuf) -> super::Commit {
        let meta = Metadata::read_object_from_file(path.to_str().unwrap().to_string())
            .expect("Read error!");

        Commit {
            meta,
            tree_id: Hash::default(),
            parent_tree_ids: vec![],
            author: AuthorSign {
                t: "".to_string(),
                name: "".to_string(),
                email: "".to_string(),
                timestamp: 0,
                timezone: "".to_string(),
            },
            committer: AuthorSign {
                t: "".to_string(),
                name: "".to_string(),
                email: "".to_string(),
                timestamp: 0,
                timezone: "".to_string(),
            },
            message: "".to_string(),
        }
    }

    ///
    #[test]
    fn test_commit_read_from_file_without_parent() {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("resources/data/test/commit-1b490ec04712d147bbe7c8b3a6d86ed4d3587a6a");

        let mut commit = get_empty_commit(path);

        commit.decode_meta().unwrap();

        assert_eq!(
            String::from("1bdbc1e723aa199e83e33ecf1bb19f874a56ebc3"),
            commit.tree_id.to_plain_str()
        );
    }

    ///
    #[test]
    fn test_commit_read_from_file_with_parent() {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("resources/data/test/commit-3b8bc1e152af7ed6b69f2acfa8be709d1733e1bb");

        let mut commit = get_empty_commit(path);

        commit.decode_meta().unwrap();

        assert_eq!(
            "9bbe4087bedef91e50dc0c1a930c1d3e86fd5f20",
            commit.tree_id.to_plain_str()
        );
    }

    ///
    #[test]
    fn test_commit_write_to_file() {
        let meta = Metadata::new(ObjectType::Commit, &vec![]);

        let author = AuthorSign {
            t: "author".to_string(),
            name: "Quanyi Ma".to_string(),
            email: "eli@patch.sh".to_string(),
            timestamp: 1649521615,
            timezone: "+0800".to_string(),
        };

        let committer = AuthorSign {
            t: "committer".to_string(),
            name: "Quanyi Ma".to_string(),
            email: "eli@patch.sh".to_string(),
            timestamp: 1649521615,
            timezone: "+0800".to_string(),
        };

        let mut commit = super::Commit {
            meta,
            tree_id:Hash::from_str("9bbe4087bedef91e50dc0c1a930c1d3e86fd5f20").unwrap(),
            parent_tree_ids: vec![
                Hash::from_str("1b490ec04712d147bbe7c8b3a6d86ed4d3587a6a").unwrap(),
            ],
            author,
            committer,
            message:"gpgsig -----BEGIN PGP SIGNATURE-----\n \n iQIzBAABCAAdFiEEanuf5/5ADLU2lvsCZL9E4tsHuXIFAmJRs88ACgkQZL9E4tsH\n uXJAmBAAtubFjLjNzIgal1/Gwy/zlpw7aQvVO2xcX3Xhbeb0UJyKvrSm/Ht19kiz\n 6Bc8ZV75mpKKip93XAljUgWgAO6Q4DUFnVA5bwF1vvhKHbgXLr+I8q+5GqmLW61U\n oBrB/3aJJ/uAxElQz5nOhgB7ztCfeKQ5egbhBXn9QGqPg/RkfQmDPYsU7evk1J0Z\n CyKinbSNe0c92qE95nURzozFb1zf0rO9NtnpYohFCEO5qyuoV4nz7npnJD4Miqy9\n IUQapeJeZC7eDvU8AWbxARrkXQkyfLSebDVcqbz7WfQz+4dhoK7jADaB48oKpR/K\n bKZDJU9a2t2nPC1ojzjQJgXZ6x4linQofBR8wE1ns3W5RoRgcBSj8dQMNH8wXa/T\n oQD6hlCJpjvbiYHuc3tSgCESI4ZU7zGpL9BAQK+C91T8CUamycF1H7TAHXdzNClR\n bWO4EeRzvwZZyIL029DYFxD2IFN7OQb5jc7JvcroIW8jUN0sMPS6jY+d0bg5pgIs\n yJjmI6qPYy7R35OElfTlw8aVSOAnVbQh7MZt6n3JUyezwK9MwbiKdAYKOLYaVaC0\n ++SY+NV4Dwe6W72KhFhxwOJQRGMfES1mRxy4n85BgqfCGy7STGSBOmon3VZEl89z\n rmvdX0JXy93hGH0oUQINsN9bzpsdaQUWVND8wAnb0+sU4LvJz90=\n =9qni\n -----END PGP SIGNATURE-----\n\nAdd gust.md and modify gitmega.md\n\nSigned-off-by: Quanyi Ma <eli@patch.sh>\n".to_string(),
        };

        commit.meta = commit.encode_metadata().unwrap();

        assert_eq!(
            "3b8bc1e152af7ed6b69f2acfa8be709d1733e1bb",
            commit.meta.id.to_plain_str()
        );

        commit
            .write_to_file("/tmp".to_string())
            .expect("Write error!");

        assert!(Path::new("/tmp/3b/8bc1e152af7ed6b69f2acfa8be709d1733e1bb").exists());
    }
}
