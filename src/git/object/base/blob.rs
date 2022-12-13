//!
//!Blob 文件对象结构体
//!
use std::cmp::Ordering;
use std::fmt::Display;
use crate::errors::GitError;
use super::Metadata;
use super::tree::{*};

/// Git Object: blob
#[derive( Eq, Debug, Hash, Clone)]
pub struct Blob {
    pub filename:String,
    pub meta: Metadata,
    
}
impl Ord for Blob {
    fn cmp(&self, other: &Self) -> Ordering {
        let o = other.filename.cmp(&self.filename);
        match o {
            Ordering::Equal => {
                other.meta.size.cmp(&self.meta.size)
            },
            _ => o,
        }
    }
}

impl PartialOrd for Blob {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let o = other.filename.cmp(&self.filename);
        match o {
            Ordering::Equal => {
                Some(other.meta.size.cmp(&self.meta.size))
            },
            _ =>  Some(o),
        }
    }
}

impl PartialEq for Blob {
    fn eq(&self, other: &Self) -> bool {
        if (self.filename.eq(&other.filename)){
            return true;
        }
        false
    }
}
///
impl Blob {


    #[allow(unused)]
    pub fn new(metadata: Metadata) -> Self {
        Self {
            meta: metadata.clone(),
            filename: String::new(),
        }
    }

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
use bstr::BString;
impl Display for Blob{
    ///为了节省输出空间 暂时只输出第一行内容
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let mut print_data:Vec<u8> = vec![];
        for value in self.meta.data.iter(){
            if *value != b'\n'{
                print_data.push(value.clone());
            }else {
                break;
            }
        }
        
        writeln!(f,"size:{}",self.meta.data.len()).unwrap();
        writeln!(f,"meta data size:{}",self.meta.size).unwrap();
        writeln!(f, "File Name: {}", self.filename ).unwrap();
        writeln!(f, "Type: Blob\n{}", BString::new(print_data) ).unwrap();
        writeln!(f, "Only Show the first line of the File...")
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


    
    use crate::git::object::Metadata;
    use crate::git::object::types::ObjectType;


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

        // if env::consts::OS == "windows" {
        //     buffer = buffer.replace(b"\r\n", b"\n");
        // }



       
    
        let data = buffer;

        let meta = Metadata::new(ObjectType::Blob,&data);

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

        assert_eq!(meta.t, crate::git::object::types::ObjectType::Blob);

        let blob = Blob {
            meta: meta.clone(),
            filename: String::new(),
        };

        assert_eq!(
            "82352c3a6a7a8bd32011751699c7a3648d1b5d3c",
            blob.meta.id.to_plain_str()
        );


        
        assert_eq!(16, blob.meta.size);
        assert_eq!(
            "# Hello Gitmega\n",
            String::from_utf8(blob.meta.data).unwrap().as_str()
        );
    }



}