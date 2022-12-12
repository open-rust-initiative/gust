//!
//! TODO:
//!  # 关于diff 的相关讨论
//! 1. 对于diff算法的选择  在myers与patience对比下明显myers更好
//! 对于 imara-diff 库， 因为包装归于
//! 
//! 
//! 
use diffs::Diff;
#[allow(dead_code)]
#[derive(Debug,Default)]
struct DeltaDiff{
   /// keep all instruction
   ops:Vec<DeltaOp>,
   old_len : usize,
   new_len : usize, 
   ///Structural Similarity,相似性
   ssam:usize,
   ssam_r:f64,
   ops_code:Vec<u8>,

}

impl DeltaDiff{
    pub fn new (old_len:usize,new_len:usize) -> Self{
        let mut _new = DeltaDiff::default();
        _new.old_len = old_len;
        _new.new_len = new_len;
        _new
    }
}



#[derive(Debug,Clone, Copy)]
enum Optype {
    DATA, // 插入的数据
    COPY, // 数据复制
}
#[allow(dead_code)]
#[derive(Debug,Clone, Copy)]
struct DeltaOp{
    /// instruction type
    ins:Optype, 
    /// data begin position
    begin: usize,
    /// data long 
    len:usize,
}
impl DeltaOp {
    /// 将Delta指令转化为git标准码
    /// 此处的data不包含数据量
    pub fn conver_to_delta(&self)-> Vec<u8>{
        let delat:Vec<u8>=Vec::new();
        match self.ins{
            Optype::DATA =>{
                let offset = self.begin;
                let size = self.len;
                let instruction :u8 = 0x80;
            },
            Optype::COPY => {},
        }
        delat
    }
    
}
impl DeltaDiff {
    fn conver_to_delta(&self)-> Vec<u8>{
        todo!();
        // let mut result  =  Vec::new();
        // for op in &self.ops {
        //     todo!()
        // }
        // vec![];
    }
}
impl Diff for DeltaDiff{
    type Error = ();
    /// offset < 2^32
    /// len < 2^24
    fn equal(&mut self, _old: usize, _new: usize, _len: usize) -> Result<(), Self::Error> {
        println!("equal {:?} {:?} {:?}", _old, _new, _len);
        self.ssam+=_len;
        self.ops.push(DeltaOp{ins:Optype::COPY,begin:_new,len:_len,});
        Ok(())
    }

    ///  insert  _len < 2 ^ 7
    fn insert(&mut self, _o: usize, _n: usize, _len: usize) -> Result<(), ()> {
        println!("insert {:?} {:?} {:?}", _o, _n, _len);
        self.ops.push(DeltaOp{ins:Optype::DATA,begin:_n,len:_len,});
        Ok(())
    }


    fn finish(&mut self) -> Result<(), Self::Error> {
        self.ssam_r = self.ssam as f64 / self.new_len as f64 ;
        Ok(())
    }
}
#[cfg(test)]
mod tests{
    use crate::git::pack::diff::DeltaDiff;

       //diff Test 
       #[test]
       fn test_imara_diff() {
        use diffs::myers;
        let a: &[usize] = &[0, 1, 3, 4, 5];
        let b: &[usize] = &[0, 1, 4, 5, 8, 9];

        let a : Vec<u8>  = vec![0, 1, 3, 4, 5];
        let b:Vec<u8> = vec![6,53,43,24,8,0, 1, 3, 4, 5];
        let mut diff = DeltaDiff::new(a.len(),b.len());
        myers::diff(&mut diff, &a, 0, a.len(), &b, 0, b.len()).unwrap();
        
        println!("{:?}",diff);
       }
   

        //diff file Test 
        #[test]
        fn test_file_diff() {
            use diffs::patience;
            use diffs::myers;
            use std::fs::File;
            use std::path::Path;
            use std::io::BufReader;
            use std::io::Read;
            use deflate::write::ZlibEncoder;
            use deflate::Compression;
            use std::io::Write;

            let  a_file = File::open(&Path::new(
                "./resources/diff/a.txt",
            ))
            .unwrap();
            let b_file = File::open(&Path::new(
                "./resources/diff/b.txt"
            ))
            .unwrap();
            let mut reader = BufReader::new(a_file);
            let mut a = Vec::new();
            reader.read_to_end(&mut a).ok();

            let mut reader = BufReader::new(b_file);
            let mut b = Vec::new();
            reader.read_to_end(&mut b).ok();



            let mut diff = DeltaDiff::new(a.len(),b.len());
            myers::diff(&mut diff, &a, 0, a.len(), &b, 0, b.len()).unwrap();

            println!("{:?}",diff);
        }
}