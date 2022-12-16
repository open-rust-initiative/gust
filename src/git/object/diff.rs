//!
//! TODO:
//!  # 关于diff 的相关讨论
//! 1. 对于diff算法的选择  在myers与patience对比下明显myers更好
//! 对于 imara-diff 库， 因为包装归于
//!
//! 每次的复制 字节大小为u8 *4 ,size 为u8*3
//!
use super::Metadata;
use crate::utils;
use diffs::myers;
use diffs::Diff;
use std::vec;

const DATA_INS_LEN: usize = 0x7f;
#[allow(dead_code)]
#[derive(Debug)]
pub struct DeltaDiff {
    /// keep all instruction
    ops: Vec<DeltaOp>,
    old_data: Metadata,
    new_data: Metadata,
    ///Structural Similarity,相似性
    ssam: usize,
    ssam_r: f64,
}

impl DeltaDiff {
    /// Diff the two Metadata , Type should be same.
    pub fn new(old: Metadata, new: Metadata) -> Self {
        assert_eq!(old.t, new.t);
        let mut _new = DeltaDiff {
            ops: vec![],
            old_data: old.clone(),
            new_data: new.clone(),

            ssam: 0,
            ssam_r: 0.00,
        };

        myers::diff(
            &mut _new,
            &old.data,
            0,
            old.data.len(),
            &new.data,
            0,
            new.data.len(),
        )
        .unwrap();
        _new
    }

    pub fn get_delta_metadata(&self) -> Vec<u8> {
        let mut result: Vec<u8> = vec![];

        // 解码后长度编码
        //BUG : 更改这里的读取
        result.append(&mut utils::write_size_encoding(self.old_data.size));
        result.append(&mut utils::write_size_encoding(self.new_data.size));

        // 编码格式
        for op in &self.ops {
            result.append(&mut self.decode_op(op));
        }
        result
    }

    fn decode_op(&self, op: &DeltaOp) -> Vec<u8> {
        let mut op_data = vec![];
        match op.ins {
            Optype::DATA => {
                assert!(op.len < 0x7f);
                let instruct = (op.len & 0x7f) as u8;
                op_data.push(instruct);
                op_data.append(&mut self.new_data.data[op.begin..op.begin + op.len].to_vec());
            }
            Optype::COPY => {
                //TODO 暂时不考虑超出范围的情况
                let mut instruct: u8 = 0x80;
                let mut offset = op.begin;
                let mut size = op.len;
                let mut copy_data = vec![];
                assert!(op.len < 0x1000000);
                for i in 0..4 {
                    let _bit = (offset & 0xff) as u8;
                    if _bit != 0 {
                        instruct |= (1 << i) as u8;
                        copy_data.push(_bit)
                    }
                    offset >>= 8;
                }
                for i in 4..7 {
                    let _bit = (size & 0xff) as u8;
                    if _bit != 0 {
                        instruct |= (1 << i) as u8;
                        copy_data.push(_bit)
                    }
                    size >>= 8;
                }
                op_data.push(instruct);
                op_data.append(&mut copy_data);
            }
        }
        op_data
    }

    pub fn get_ssam_rate(&self) -> f64 {
        self.ssam_r
    }
}
impl Diff for DeltaDiff {
    type Error = ();
    /// offset < 2^32
    /// len < 2^24
    fn equal(&mut self, _old: usize, _new: usize, _len: usize) -> Result<(), Self::Error> {
        // 暂时未支持长度过大时的拆分情况
        assert!(_old < (1 << 33));
        assert!(_len < (1 << 25));
        self.ssam += _len;
        self.ops.push(DeltaOp {
            ins: Optype::COPY,
            begin: _old,
            len: _len,
        });
        Ok(())
    }

    ///  insert  _len < 2 ^ 7
    fn insert(&mut self, _old: usize, _new: usize, _len: usize) -> Result<(), ()> {
        // 暂时未支持长度过大时的拆分情况

        // // | 0xxxxxxx | |data| |
        let mut len = _len;
        let mut new = _new;
        if _len > DATA_INS_LEN {
            while len > DATA_INS_LEN {
                self.ops.push(DeltaOp {
                    ins: Optype::DATA,
                    begin: new,
                    len: DATA_INS_LEN,
                });
                len -= DATA_INS_LEN;
                new += DATA_INS_LEN;
            }
            self.ops.push(DeltaOp {
                ins: Optype::DATA,
                begin: new,
                len: len,
            });
        } else {
            self.ops.push(DeltaOp {
                ins: Optype::DATA,
                begin: _new,
                len: _len,
            });
        }

        Ok(())
    }

    fn finish(&mut self) -> Result<(), Self::Error> {
        // compute the ssam rate when finish the diff process.
        self.ssam_r = self.ssam as f64 / self.new_data.data.len() as f64;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
enum Optype {
    DATA, // 插入的数据
    COPY, // 数据复制
}

#[derive(Debug, Clone, Copy)]
struct DeltaOp {
    /// instruction type
    ins: Optype,
    /// data begin position
    begin: usize,
    /// data long
    len: usize,
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use super::DeltaDiff;
    use crate::{
        git::{
            object::{types::ObjectType, Metadata},
            pack::Pack,
        },
        utils,
    };
    use bstr::ByteSlice;

    /// 通过两个metadata 来进行对后者No.2的压缩
    /// 首先，需要两个是相同的类型(ObjectType)
    /// 先确定要进行什么类型的压缩，
    ///    1. ofs-object 将以No.1为base压缩为ofs-object,offset 来标识负距离上的object开头
    ///    2. ref-object 将以No.1为base， 以hash值作为标识
    /// 两种delta的共性：都需要未压缩的header编码。ofs 是sized编码的开头。ref是hash的20位u8
    /// 1，
    ///
    #[test]
    fn test_metadata_diff_ofs_delta() {
        let m1 = Metadata::read_object_from_file(
            "./resources/diff/16ecdcc8f663777896bd39ca025a041b7f005e".to_string(),
        )
        .unwrap();
        let mut m2 = Metadata::read_object_from_file(
            "./resources/diff/bee0d45f981adf7c2926a0dc04deb7f006bcc3".to_string(),
        )
        .unwrap();
        let diff = DeltaDiff::new(m1.clone(), m2.clone());
        println!("{:?}", diff);
        let meta_vec1 = m1.convert_to_vec().unwrap();

        // 对于offset的
        // 不需要压缩的size
        let offset_head = utils::write_offset_encoding(meta_vec1.len() as u64);

        // 需要压缩的指令data
        let zlib_data = diff.get_delta_metadata();
        m2.change_to_delta(ObjectType::OffsetDelta, zlib_data, offset_head);

        // 排好序后直接把metadata按顺序放入Vec就行了
        let meta_vec = vec![m1, m2];
        let mut _pack = Pack::default();
        let pack_file_data = _pack.encode(Some(meta_vec));
        //_pack
        let mut file = std::fs::File::create("delta_ofs.pack").expect("create failed");
        file.write_all(pack_file_data.as_bytes())
            .expect("write failed");
        Pack::decode_file("delta_ofs.pack");
    }

    #[test]
    fn test_metadata_diff_ref_delta() {
        let m1 = Metadata::read_object_from_file(
            "./resources/diff/16ecdcc8f663777896bd39ca025a041b7f005e".to_string(),
        )
        .unwrap();
        let mut m2 = Metadata::read_object_from_file(
            "./resources/diff/bee0d45f981adf7c2926a0dc04deb7f006bcc3".to_string(),
        )
        .unwrap();
        let diff = DeltaDiff::new(m1.clone(), m2.clone());
        println!("{:?}", diff);

        //不需要压缩
        let offset_head = m1.id.0.to_vec();
        assert!(offset_head.len() == 20);

        //需要压缩
        let zlib_data = diff.get_delta_metadata();
        m2.change_to_delta(ObjectType::HashDelta, zlib_data, offset_head);

        let meta_vec = vec![m1, m2];
        let mut _pack = Pack::default();
        let pack_file_data = _pack.encode(Some(meta_vec));
        //_pack
        let mut file = std::fs::File::create("delta_ref.pack").expect("create failed");
        file.write_all(pack_file_data.as_bytes())
            .expect("write failed");
        Pack::decode_file("delta_ref.pack");
    }
}
