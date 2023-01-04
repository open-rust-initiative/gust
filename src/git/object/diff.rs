//!
//!
//!
//!
//!
use std::vec;

use diffs::Diff;
use diffs::myers;

use crate::git::utils;
use crate::git::object::metadata::MetaData;

const DATA_INS_LEN: usize = 0x7f;

#[allow(dead_code)]
#[derive(Debug)]
pub struct DeltaDiff {
    ops: Vec<DeltaOp>,
    old_data: MetaData,
    new_data: MetaData,
    ssam: usize,
    ssam_r: f64,
}

impl DeltaDiff {
    /// Diff the two Metadata , Type should be same.
    /// Return the DeltaDiff struct.
    pub fn new(old_md: MetaData, new_md: MetaData) -> Self {
        let mut delta_diff = DeltaDiff {
            ops: vec![],
            old_data: old_md.clone(),
            new_data: new_md.clone(),

            ssam: 0,
            ssam_r: 0.00,
        };

        myers::diff(
            &mut delta_diff,
            &old_md.data,
            0,
            old_md.data.len(),
            &new_md.data,
            0,
            new_md.data.len(),
        ).unwrap();

        delta_diff
    }

    ///
    ///
    pub fn get_delta_metadata(&self) -> Vec<u8> {
        let mut result: Vec<u8> = vec![];

        result.append(&mut utils::write_size_encoding(self.old_data.size));
        result.append(&mut utils::write_size_encoding(self.new_data.size));

        for op in &self.ops {
            result.append(&mut self.decode_op(op));
        }

        result
    }

    ///
    /// Decode the DeltaOp to Vec<u8>
    fn decode_op(&self, op: &DeltaOp) -> Vec<u8> {
        let mut op_data = vec![];

        match op.ins {
            Optype::DATA => {
                let instruct = (op.len & 0x7f) as u8;
                op_data.push(instruct);
                op_data.append(&mut self.new_data.data[op.begin..op.begin + op.len].to_vec());
            }

            Optype::COPY => {
                let mut instruct: u8 = 0x80;
                let mut offset = op.begin;
                let mut size = op.len;
                let mut copy_data = vec![];

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

    ///
    pub fn get_ssam_rate(&self) -> f64 {
        self.ssam_r
    }
}

impl Diff for DeltaDiff {
    type Error = ();

    ///
    fn equal(&mut self, _old: usize, _new: usize, _len: usize) -> Result<(), Self::Error> {
        self.ssam += _len;
        if let Some(tail) = self.ops.last_mut() {
            if tail.begin + tail.len == _old && tail.ins == Optype::COPY {
                tail.len += _len;
            } else {
                self.ops.push(DeltaOp {
                    ins: Optype::COPY,
                    begin: _old,
                    len: _len,
                });
            }
        } else {
            self.ops.push(DeltaOp {
                ins: Optype::COPY,
                begin: _old,
                len: _len,
            });
        }

        Ok(())
    }

    ///
    ///
    fn insert(&mut self, _old: usize, _new: usize, _len: usize) -> Result<(), ()> {
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
                len,
            });
        } else {
            if let Some(tail) = self.ops.last_mut() {
                if tail.begin + tail.len == _new
                    && tail.ins == Optype::DATA
                    && tail.len + _len < DATA_INS_LEN
                {
                    tail.len += _len;
                } else {
                    self.ops.push(DeltaOp {
                        ins: Optype::DATA,
                        begin: new,
                        len: len,
                    });
                }
            } else {
                self.ops.push(DeltaOp {
                    ins: Optype::DATA,
                    begin: new,
                    len: len,
                });
            }
        }

        Ok(())
    }

    fn finish(&mut self) -> Result<(), Self::Error> {
        // compute the ssam rate when finish the diff process.
        self.ssam_r = self.ssam as f64 / self.new_data.data.len() as f64;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Optype {
    DATA,
    COPY,
}

#[derive(Debug, Clone, Copy)]
struct DeltaOp {
    ins: Optype,
    begin: usize,
    len: usize,
}

#[cfg(test)]
mod tests {
    use std::io::Write;
    use std::path::PathBuf;

    use bstr::ByteSlice;

    use crate::{
        git::{
            object::metadata::MetaData,
            object::types::ObjectType,
            pack::Pack,
            utils,
        },
    };

    use super::DeltaDiff;

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
        let mut m1_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        m1_path.push("resources/diff/16ecdcc8f663777896bd39ca025a041b7f005e");

        let mut m2_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        m2_path.push("resources/diff/bee0d45f981adf7c2926a0dc04deb7f006bcc3");

        let m1 = MetaData::read_object_from_file(
            m1_path.to_str().unwrap().to_string()).unwrap();
        let mut m2 = MetaData::read_object_from_file(
            m2_path.to_str().unwrap().to_string()).unwrap();

        let diff = DeltaDiff::new(m1.clone(), m2.clone());
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
        let mut m1_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        m1_path.push("resources/diff/16ecdcc8f663777896bd39ca025a041b7f005e");

        let mut m2_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        m2_path.push("resources/diff/bee0d45f981adf7c2926a0dc04deb7f006bcc3");

        let m1 = MetaData::read_object_from_file(
            m1_path.to_str().unwrap().to_string()).unwrap();
        let mut m2 = MetaData::read_object_from_file(
            m2_path.to_str().unwrap().to_string()).unwrap();

        let diff = DeltaDiff::new(m1.clone(), m2.clone());

        //不需要压缩
        let offset_head = m1.id.0.to_vec();
        assert_eq!(offset_head.len(), 20);

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
