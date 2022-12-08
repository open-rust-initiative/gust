use std::io::Write;
use std::str::FromStr;
use bstr::ByteSlice;
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

    pub fn encode(&mut self,meta_vec :Option<Vec<Metadata>>) -> Vec<u8> {
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
        
        self.signature = checksum.clone();
        result.append(&mut checksum.0.to_vec());
        result
    }
    /// Pack the loose object from the Given string .
    /// `obj_path`: the vector of the Hash value of the loose object 
    /// `loose_root_path` : loose objects' root path 
    /// `target_path` : the pack file store path 
    pub fn pack_loose_files(obj_path:Vec<String>,loose_root_path:&str,target_path: &str) -> Self{

        let mut meta_vec = vec![];
        for path in &obj_path {
            let hash_value = Hash::from_str(path).unwrap();
            let loose_path = format!("{}/{}/{}",loose_root_path,hash_value.to_folder(),hash_value.to_filename());
            let _meta=Metadata::read_object_from_file(loose_path);
            match _meta {
                Ok(meta) => meta_vec.push(meta),
                Err(e) => eprintln!("{}",e),
            }
        }

        // if meta_vec.len() != obj_path.len(){ 
        //     return false;
        // }
        let mut pack= Pack::default();
        let pack_file_data =pack.encode( Some(meta_vec));

        let pack_file_name = format!("{}/pack-{}.pack",target_path,pack.signature.to_plain_str());
        print!("to——file: {}",pack_file_name);
        let mut file = std::fs::File::create(pack_file_name).expect("create failed");
        file.write_all(pack_file_data.as_bytes()).expect("write failed");



        pack
    }
    /// Pack the loose object in a dir ,such as the `.git/object/pack`<br>
    /// It can auto find the loose object follow the position like below:
    /// ```
    /// ./in：loose_root/aa/bbbbbbbbbbbbbbbbbb
    /// ```
    /// ,The object Hash is `aabbbbbbbbbbbbbbbbbb`
    /// - in：loose_root  : loose object root dir
    /// - in: target_path : The pack file dir to store
    #[allow(unused)]
    pub fn pack_from_dir(loose_root_path:&str,target_path: &str)-> Self{
        let loose_root = std::path::PathBuf::from(loose_root_path) ;
        let mut loose_vec=Vec::new();
        // 打开loose 根目录
        let paths = std::fs::read_dir(&loose_root).unwrap();
        // 暂时保存根目录作为 Path buff
        let mut loose_file = loose_root.clone(); 
        // loose_file= ./root
        // 遍历目录下的hash前两位(1b)的子文件夹
        for path in paths {
            if let Ok(hash_2) = path{

                //the first 1 b
                let file_name1 =String::from(hash_2.file_name().to_str().unwrap());

                // 判断只有两位且是文件夹
                let is_dir = hash_2.file_type().unwrap().is_dir();
                if  is_dir && (file_name1.len()==2) {

                    loose_file.push(file_name1.clone());
                    //loose_file = ./root/xx
                    let loose_s = std::fs::read_dir(&loose_file).unwrap();

                    //再打开子文件夹 此目录下即为保存的loose object文件
                    for loose_path in loose_s{
                        if let Ok(loose_path) = loose_path {
                            let file_name2 =String::from(loose_path.file_name().to_str().unwrap());
                            loose_file.push(file_name2.clone());
                            //loose_file = ./root/xx/xxxxxxxxxxxxxxxxxxxx
                            //将object提取hash值并放入vec
                            loose_vec.push(
                                Hash::from_str(
                                    &(file_name1.clone()+&file_name2)
                                ).unwrap().to_plain_str()
                            );
                            loose_file.pop();// pop path buf 
                        }
                    }
                    loose_file.pop();
                }else {
                    continue;
                }
            }
        }
        Pack::pack_loose_files(loose_vec,loose_root_path,target_path)
        
    
    }
}

#[cfg(test)]
mod tests {
    
    use std::{fs::File};
    use std::io::Write;
    use std::path::Path;

    use bstr::{ByteSlice};

    
    use crate::git::pack::{decode::ObjDecodedMap, Pack};

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
        let mut decoded_pack = match Pack::decode(&mut pack_file) {
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
    
    /// 将一些loose object打包为 pack文件
    /// 只需要给出loose文件的根目录 目标根目录 和 loose 文件的hash字符串即可
    #[test]
    fn test_loose_pack(){
        let mut loose_vec=Vec::new();
        loose_vec.push(format!("5f413c76a2893bb1ff83d7c2b507a9cab30bd585"));
        loose_vec.push(format!("8bb783eb532d4936248f9084821af2bb309f29e7"));
        loose_vec.push(format!("79dc1608dba888e0378ff21591dc646c8afe4e0a"));
        loose_vec.push(format!("ce70a618efa88992a4c4bdf22ebd832b24acf374"));
        let loose_root =  "./resources/loose";
        let target_path = "./resources/pack_g";
        let pack = Pack::pack_loose_files(loose_vec, loose_root, target_path);
        Pack::decode_file(&format!("{}/pack-{}.pack",target_path,pack.signature.to_plain_str()));
    }

    /// 只需要给定loose 的根目录 则自动读取所有loose的文件并打包至指定文件夹
    #[test]
    fn test_loose_pack_from_dir(){
        let loose_root =  "./resources/loose";
        let target_path = "./resources/pack_g";
        // 解析过程
        let pack = Pack::pack_from_dir( loose_root, target_path);
        Pack::decode_file(&format!("{}/pack-{}.pack",target_path,pack.signature.to_plain_str()));
    }

}