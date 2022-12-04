use super::Pack;
use std::convert::TryFrom;

impl Pack{

    #[allow(unused)]
    pub fn encode(&self)->Vec<u8>{
        use sha1::{Digest, Sha1};
        let mut obj_vec = vec![];
        for (key ,value ) in self.result.by_hash.iter(){
            obj_vec.push(value.to_metadata());
        }
        let mut result:Vec<u8>=vec![b'P',b'A',b'C',b'K',0,0,0,2];
        
        let mut all_num =obj_vec.len();
        assert!( all_num< (1<<32) );//TODO: GitError < 4G
        result.push( (all_num >> 24 )  as u8);
        result.push( (all_num >> 16 )  as u8);
        result.push( (all_num >> 8  )  as u8);
        result.push( (all_num) as u8);

        for metadata in obj_vec{
           result.append(&mut metadata.convert_to_vec().unwrap());
        }
        let result_hash = Sha1::new().chain(&result).finalize();
        let mut checksum = <[u8; 20]>::try_from(result_hash.as_slice()).unwrap();
        result.append(&mut checksum.to_vec());
        result
    }
    //PackObjectCache
}

#[cfg(test)]
mod tests{
    use std::fs::File;
    use std::io::Write;
    use std::path::Path;

    use bstr::ByteSlice;
    #[test]
    fn test_imara_diff(){
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
        let diff_str = diff(Algorithm::Histogram, &input, UnifiedDiffBuilder::new(&input));
        println!("{}",diff_str);
        
    }
    #[test]
    fn test_a_real_pack(){
        let mut pack_file = File::open(&Path::new(
            //".git/objects/aa/36c1e0d709f96d7b356967e16766bafdf63a75",
            "./resources/test1/pack-1d0e6c14760c956c173ede71cb28f33d921e232f.pack",
        ))
        .unwrap();
        use super::super::Pack;
        let decoded_pack = match Pack::decode(&mut pack_file){
            Ok(f)=> f,
            Err(e) => panic!("{}",e.to_string()),
        };
        assert_eq!(*b"PACK", decoded_pack.head);
        assert_eq!(2, decoded_pack.version);

        let result  = decoded_pack.encode();
        let mut file = std::fs::File::create("data.txt").expect("create failed");
        file.write_all(result.as_bytes()).expect("write failed");

        println!("data written to file" );
    }
    #[test]
    fn test_output_pack(){
        let mut pack_file = File::open(&Path::new(
            "./data.txt",
        ))
        .unwrap();
        use super::super::Pack;
        let decoded_pack = match Pack::decode(&mut pack_file){
            Ok(f)=> f,
            Err(e) => panic!("{}",e.to_string()),
        };
        assert_eq!(*b"PACK", decoded_pack.head);
        assert_eq!(2, decoded_pack.version);

    }
    #[test]
    fn dex_number(){
        let all_num:usize = 0x100f1109;
        println!("{:x}",(all_num >> 24 )  as u8); 
        println!("{:x}",(all_num >> 16 )  as u8);
        println!("{:x}",(all_num >> 8 )  as u8);
        println!("{:x}",(all_num  ) as u8);
    }
}