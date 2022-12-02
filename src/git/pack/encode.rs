#[cfg(test)]
mod tests{
    use std::fs::File;
    use std::path::Path;
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
    }
}