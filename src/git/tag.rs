//!
//!
//!
//!
//!
//!
//!

use bstr::ByteSlice;

use crate::errors::GitError;

use crate::git::id::ID;
use crate::git::{Metadata, Type};
use crate::git::hash::Hash;
use crate::git::sign::AuthorSign;

/// Git Object: tag
#[allow(unused)]
pub struct Tag {
    pub meta: Metadata,
    pub object: ID,
    pub t: Type,
    pub tag: String,
    pub tagger: AuthorSign,
    pub message: String,
}

///
impl Tag {
    ///
    #[allow(unused)]
    fn decode_metadata(&mut self) -> Result<(), GitError> {
        let mut data = self.meta.data.clone();

        let object_begin = data.find_byte(0x20).unwrap();
        let object_end = data.find_byte(0x0a).unwrap();
        self.object = ID::from_string(data[object_begin + 1..object_end].to_str().unwrap());
        data = data[object_end + 1..].to_vec();

        let type_begin = data.find_byte(0x20).unwrap();
        let type_end = data.find_byte(0x0a).unwrap();
        self.t = Type::from_string(data[type_begin + 1..type_end].to_str().unwrap()).unwrap();
        data = data[type_end + 1..].to_vec();

        let tag_begin = data.find_byte(0x20).unwrap();
        let tag_end = data.find_byte(0x0a).unwrap();
        self.tag = data[tag_begin + 1..tag_end].to_str().unwrap().parse().unwrap();
        data = data[type_end..].to_vec();

        let tagger = data.find("tagger").unwrap();
        let tagger_data = data[.. data.find_byte(0x0a).unwrap()].to_vec();
        self.tagger.decode_from_data(tagger_data)?;
        data = data[data.find_byte(0x0a).unwrap() + 1..].to_vec();

        self.message = data[data.find_byte(0x0a).unwrap()..].to_vec().to_str().unwrap().to_string();

        Ok(())
    }

    ///
    #[allow(unused)]
    fn encode_metadata(&self) -> Result<Metadata, ()> {
        let mut data = Vec::new();

        data.extend_from_slice("object".as_bytes());
        data.extend_from_slice(0x20u8.to_be_bytes().as_ref());
        data.extend_from_slice(self.object.to_string().as_bytes());
        data.extend_from_slice(0x0au8.to_be_bytes().as_ref());

        data.extend_from_slice("type".as_bytes());
        data.extend_from_slice(0x20u8.to_be_bytes().as_ref());
        data.extend_from_slice(self.t.to_string().as_bytes());
        data.extend_from_slice(0x0au8.to_be_bytes().as_ref());

        data.extend_from_slice("tag".as_bytes());
        data.extend_from_slice(0x20u8.to_be_bytes().as_ref());
        data.extend_from_slice(self.tag.as_bytes());
        data.extend_from_slice(0x0au8.to_be_bytes().as_ref());

        data.extend_from_slice(self.tagger.encode_to_data().unwrap().as_ref());
        data.extend_from_slice(0x0au8.to_be_bytes().as_ref());
        data.extend_from_slice(self.message.as_bytes());

        Ok(
            Metadata {
                t: Type::Tag,
                h: Hash::Sha1,
                id: ID::from_vec(Type::Tag, &mut data),
                size: data.len(),
                data,
            })
    }

    ///
    #[allow(unused)]
    fn write_to_file(&self, root_path: String) -> Result<String, GitError> {
        self.meta.write_to_file(root_path)
    }
}

///
#[cfg(test)]
mod tests {
    use std::env;
    use std::path::Path;
    use std::path::PathBuf;

    ///
    #[test]
    fn test_commit_read_from_file_without_parent() {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("resources/data/test/commit-1b490ec04712d147bbe7c8b3a6d86ed4d3587a6a");

        let meta = crate::git::Metadata::read_object_from_file(path.to_str().unwrap().to_string())
            .expect("Read error!");

        assert_eq!(crate::git::Type::Commit, meta.t);
        assert_eq!(1065, meta.size);
        assert_eq!(
            "1b490ec04712d147bbe7c8b3a6d86ed4d3587a6a",
            meta.id.to_string()
        );

        let mut commit = crate::git::commit::Commit {
            meta,
            tree_id: crate::git::id::ID { bytes: vec![], hash: "".to_string() },
            parent_tree_ids: vec![],
            author: crate::git::sign::AuthorSign {
                t: "".to_string(),
                name: "".to_string(),
                email: "".to_string(),
                timestamp: 0,
                timezone: "".to_string()
            },
            committer: crate::git::sign::AuthorSign {
                t: "".to_string(),
                name: "".to_string(),
                email: "".to_string(),
                timestamp: 0,
                timezone: "".to_string()
            },
            message: "".to_string()
        };

        commit.decode_meta().unwrap();

        assert_eq!("1bdbc1e723aa199e83e33ecf1bb19f874a56ebc3", commit.tree_id.hash);

    }

    ///
    #[test]
    fn test_commit_read_from_file_with_parent() {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("resources/data/test/commit-3b8bc1e152af7ed6b69f2acfa8be709d1733e1bb");

        let meta = crate::git::Metadata::read_object_from_file(path.to_str().unwrap().to_string())
            .expect("Read error!");

        assert_eq!(crate::git::Type::Commit, meta.t);
        assert_eq!(1126, meta.size);
        assert_eq!(
            "3b8bc1e152af7ed6b69f2acfa8be709d1733e1bb",
            meta.id.to_string()
        );

        let mut commit = crate::git::commit::Commit {
            meta,
            tree_id: super::ID { bytes: vec![], hash: "".to_string() },
            parent_tree_ids: vec![],
            author: crate::git::sign::AuthorSign {
                t: "".to_string(),
                name: "".to_string(),
                email: "".to_string(),
                timestamp: 0,
                timezone: "".to_string()
            },
            committer: crate::git::sign::AuthorSign {
                t: "".to_string(),
                name: "".to_string(),
                email: "".to_string(),
                timestamp: 0,
                timezone: "".to_string()
            },
            message: "".to_string()
        };

        commit.decode_meta().unwrap();

        assert_eq!("9bbe4087bedef91e50dc0c1a930c1d3e86fd5f20", commit.tree_id.to_string());
    }

    ///
    #[test]
    fn test_commit_write_to_file() {
        let meta = crate::git::Metadata {
            t: super::Type::Commit,
            h: super::Hash::Sha1,
            size: 0,
            id: super::ID { bytes: vec![], hash: "".to_string() },
            data: vec![],
        };

        let author = crate::git::sign::AuthorSign {
            t: "author".to_string(),
            name: "Quanyi Ma".to_string(),
            email: "eli@patch.sh".to_string(),
            timestamp: 1649521615,
            timezone: "+0800".to_string()
        };

        let committer = crate::git::sign::AuthorSign {
            t: "committer".to_string(),
            name: "Quanyi Ma".to_string(),
            email: "eli@patch.sh".to_string(),
            timestamp: 1649521615,
            timezone: "+0800".to_string()
        };

        let mut commit = crate::git::commit::Commit {
            meta,
            tree_id: super::ID::from_string("9bbe4087bedef91e50dc0c1a930c1d3e86fd5f20"),
            parent_tree_ids: vec![
                super::ID::from_string("1b490ec04712d147bbe7c8b3a6d86ed4d3587a6a"),
            ],
            author,
            committer,
            message:"gpgsig -----BEGIN PGP SIGNATURE-----\n \n iQIzBAABCAAdFiEEanuf5/5ADLU2lvsCZL9E4tsHuXIFAmJRs88ACgkQZL9E4tsH\n uXJAmBAAtubFjLjNzIgal1/Gwy/zlpw7aQvVO2xcX3Xhbeb0UJyKvrSm/Ht19kiz\n 6Bc8ZV75mpKKip93XAljUgWgAO6Q4DUFnVA5bwF1vvhKHbgXLr+I8q+5GqmLW61U\n oBrB/3aJJ/uAxElQz5nOhgB7ztCfeKQ5egbhBXn9QGqPg/RkfQmDPYsU7evk1J0Z\n CyKinbSNe0c92qE95nURzozFb1zf0rO9NtnpYohFCEO5qyuoV4nz7npnJD4Miqy9\n IUQapeJeZC7eDvU8AWbxARrkXQkyfLSebDVcqbz7WfQz+4dhoK7jADaB48oKpR/K\n bKZDJU9a2t2nPC1ojzjQJgXZ6x4linQofBR8wE1ns3W5RoRgcBSj8dQMNH8wXa/T\n oQD6hlCJpjvbiYHuc3tSgCESI4ZU7zGpL9BAQK+C91T8CUamycF1H7TAHXdzNClR\n bWO4EeRzvwZZyIL029DYFxD2IFN7OQb5jc7JvcroIW8jUN0sMPS6jY+d0bg5pgIs\n yJjmI6qPYy7R35OElfTlw8aVSOAnVbQh7MZt6n3JUyezwK9MwbiKdAYKOLYaVaC0\n ++SY+NV4Dwe6W72KhFhxwOJQRGMfES1mRxy4n85BgqfCGy7STGSBOmon3VZEl89z\n rmvdX0JXy93hGH0oUQINsN9bzpsdaQUWVND8wAnb0+sU4LvJz90=\n =9qni\n -----END PGP SIGNATURE-----\n\nAdd gust.md and modify gitmega.md\n\nSigned-off-by: Quanyi Ma <eli@patch.sh>\n".to_string(),
        };

        commit.meta = commit.encode_metadata().unwrap();

        assert_eq!("3b8bc1e152af7ed6b69f2acfa8be709d1733e1bb", commit.meta.id.to_string());

        commit.write_to_file("/tmp".to_string()).expect("Write error!");

        assert!(Path::new("/tmp/3b/8bc1e152af7ed6b69f2acfa8be709d1733e1bb").exists());
    }

    ///
    #[test]
    fn test_tag_read_from_file() {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("resources/data/test/tag-e5c324b03b72b26f11557c4955c6d17c68dc8595");

        let meta = crate::git::Metadata::read_object_from_file(path.to_str().unwrap().to_string())
            .expect("Read error!");

        assert_eq!(crate::git::Type::Tag, meta.t);
        assert_eq!(976, meta.size);
        assert_eq!("e5c324b03b72b26f11557c4955c6d17c68dc8595", meta.id.to_string());

        let mut tag = super::Tag {
            meta,
            object: super::ID { bytes: vec![], hash: "".to_string() },
            t: super::Type::Commit,
            tag: "".to_string(),
            tagger: super::AuthorSign {
                t: "".to_string(),
                name: "".to_string(),
                email: "".to_string(),
                timestamp: 0,
                timezone: "+0000".to_string()
            },
            message: "".to_string(),
        };

        tag.decode_metadata().unwrap();

        assert_eq!("6414e45babf0bdd043ba40d31123053cfebef26c", tag.object.to_string());
        assert_eq!("commit", tag.t.to_string());
        assert_eq!("v1.1.0", tag.tag);
        assert_eq!(1653037847, tag.tagger.timestamp);
    }

    ///
    #[test]
    fn test_tag_write_to_file() {
        let meta = crate::git::Metadata {
            t: super::Type::Tag,
            h: super::Hash::Sha1,
            size: 0,
            id: super::ID { bytes: vec![], hash: "".to_string() },
            data: vec![],
        };

        let tagger = crate::git::sign::AuthorSign {
            t: "tagger".to_string(),
            name: "Quanyi Ma".to_string(),
            email: "eli@patch.sh".to_string(),
            timestamp: 1653037847,
            timezone: "+0800".to_string()
        };

        let mut tag = super::Tag {
            meta,
            object: crate::git::id::ID::from_string("6414e45babf0bdd043ba40d31123053cfebef26c"),
            t: super::Type::Commit,
            tag: "v1.1.0".to_string(),
            tagger,
            message: "\nIt's a lastest object\n-----BEGIN PGP SIGNATURE-----\n\niQIzBAABCAAdFiEEanuf5/5ADLU2lvsCZL9E4tsHuXIFAmKHWxcACgkQZL9E4tsH\nuXIeFhAAtX+foSvc7/1lb98+QfRjHcpO+LX+LroTaq/QGOTX/2gE+tHD2TJAga1I\nVqDEz8fh8AE366FC7UCjCb5nvsCCox2htzbIxAjsc9L/JckWtxl6WOa/5OZssrDQ\nFtX39BNNl+4TfNn/z1XV+28c9yB1N5HSoP2gzdLoASw3y9n6E0FyzLdoXPILgmJI\nL4DAG/OFkixK+I+TsK+6995497h9BCi3x30dOjfxZS9ptiKhqWulbkflvvM9Cnie\n7obXYmnoe0jBjSfO5GgJlOYcLzE9MMYYzIx47/4lcrCbQXnojkW3KV03PEXGfRCL\nw/y8oBHVvNVRF0Jn+o7F+mzIrbF6Ufku63MfRf7WmbbS3B63CILEjNyuOFoe8mDb\nrmAUffzQSrgnvBk+g01slb6Q+q7Urw6wqHtBPn3ums/inHE9ymTqS7ffmRifUfR8\nD8LvhwpSUI7BdiN6HznRFPxMXzohYIqAJbUltjr4Q7qw/kJI+305Xcs1U5AUIaOp\n77p2UFHRVoMM5mpPOCSwsVJ6cSuOjWXf9afcNMrhgclKefM0aXXnd2p5zTUEe99T\nlAtXHuprRwxtSQUzHxJCdGlUGRGRR2aS9W984SNDVmcegnOIrZD2pVm/tjDwVex5\nMuAuKHr8et1EKyvKCnta6USq7WC2l6RdsCaAYzSTQ7ljEi9A+6Q=\n=/9g0\n-----END PGP SIGNATURE-----\n".to_string(),
        };

        tag.meta = tag.encode_metadata().unwrap();
        assert_eq!("e5c324b03b72b26f11557c4955c6d17c68dc8595", tag.meta.id.to_string());

        tag.write_to_file("/tmp".to_string()).expect("Write error!");
        assert!(Path::new("/tmp/e5/c324b03b72b26f11557c4955c6d17c68dc8595").exists());

    }
}