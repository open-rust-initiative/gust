use crate::git::Metadata;

use super::super::hash::Hash;
use super::Pack;
///
/// Pack类的encode函数，将解析出的pack或其他途径生成的pack生成对应的文件
impl Pack {
    #[allow(unused)]
    /// Pack 结构体的`encode`函数
    ///  > 若输出的meta_vec ==None 则需要pack结构体是完整有效的，或者至少其中的PackObjectCache不为空
    ///  > 若输入的meta_vec不为None 则按照该vec进行encode
    /// # Examples
    /// ```
    ///   let result:Vec<u8> = decoded_pack.encode(None);
    ///     //or
    ///   let metadata_vec :Vec<Metadata> = ...;// Get a list of metadata
    ///   let result:Vec<u8> = Pack::default().encode(metadata_vec);  
    /// ```

    pub fn encode(&self,meta_vec :Option<Vec<Metadata>>) -> Vec<u8> {
        use sha1::{Digest, Sha1};
        let mut obj_vec = vec![];
        match meta_vec {
            Some(a) => obj_vec = a,
            None => {
                for (key, value) in self.result.by_hash.iter() {
                    obj_vec.push(value.to_metadata());
                }
            },
        }

        let mut result: Vec<u8> = 
        vec![b'P', b'A', b'C', b'K',  // The logotype of the Pack File
             0   , 0   , 0   , 2   ,];// THe Version  of the Pack File 

        let mut all_num = obj_vec.len();
        assert!(all_num < (1 << 32)); //TODO: GitError < 4G
        //Encode the number of object  into file
        result.push((all_num >> 24) as u8);
        result.push((all_num >> 16) as u8);
        result.push((all_num >> 8) as u8);
        result.push((all_num) as u8);

        for metadata in obj_vec {
            result.append(&mut metadata.convert_to_vec().unwrap());
        }

        let mut checksum = Hash::new(&result);
        result.append(&mut checksum.0.to_vec());
        result
    }
    
}

#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::io::Write;
    use std::path::Path;

    use bstr::{ByteSlice};

    use crate::git::pack::decode::ObjDecodedMap;

    //diff Test 
    #[test]
    fn test_imara_diff() {
        use imara_diff::intern::InternedInput;
        use imara_diff::{diff, Algorithm, UnifiedDiffBuilder};
        let before = r#"fn foo() -> Bar {
        let mut foo = 2;
        foo *= 50;
        println!("hello world")
        }"#;
        let after = r#"// lorem ipsum
        fn foo() -> Bar {
        let mut foo = 2;
        foo *= 50;
        println!("hello world");
        println!("{foo}");
        }
        // foo
        "#;
        let input = InternedInput::new(before, after);
        let diff_str = diff(
            Algorithm::Histogram,
            &input,
            UnifiedDiffBuilder::new(&input),
        );
        println!("{}", diff_str);
    }

    //
    #[test]
    fn test_a_real_pack_de_en() {
        let mut pack_file = File::open(&Path::new(
            "./resources/test1/pack-1d0e6c14760c956c173ede71cb28f33d921e232f.pack",
        ))
        .unwrap();
        use super::super::Pack;
        let decoded_pack = match Pack::decode(&mut pack_file) {
            Ok(f) => f,
            Err(e) => panic!("{}", e.to_string()),
        };
        assert_eq!(*b"PACK", decoded_pack.head);
        assert_eq!(2, decoded_pack.version);


        let result = decoded_pack.encode(None);
        let mut file = std::fs::File::create("data.pack").expect("create failed");
        file.write_all(result.as_bytes()).expect("write failed");

        println!("data written to file");
        // 将生成的pack文件重新进行一遍解析，以此验证生成文件的正确性
        let mut pack_file = File::open(&Path::new("./data.pack")).unwrap();
        let decoded_pack = match Pack::decode(&mut pack_file) {
            Ok(f) => f,
            Err(e) => panic!("{}", e.to_string()),
        };
        assert_eq!(*b"PACK", decoded_pack.head);
        assert_eq!(2, decoded_pack.version);

        let mut result = ObjDecodedMap::default();
        result.update_from_cache(&decoded_pack.result);
        
        for (key, value) in result._map_hash.iter() {
            println!("*********************");
            println!("Hash :{}", key);
            println!("{}", value);
        }




    }

    #[test]
    fn dex_number() {
        let all_num: usize = 0x100f1109;
        println!("{:x}", (all_num >> 24) as u8);
        println!("{:x}", (all_num >> 16) as u8);
        println!("{:x}", (all_num >> 8) as u8);
        println!("{:x}", (all_num) as u8);
    }
}
