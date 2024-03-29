//!
//!
//!
//!

use std::{
    fs::File,
    io::{self, Read, Seek, SeekFrom},
    path::PathBuf,
    str::FromStr,
    vec,
};

use flate2::read::ZlibDecoder;

use crate::git::errors::GitError;
use crate::git::hash::Hash;

const TYPE_BITS: u8 = 3;
const VAR_INT_ENCODING_BITS: u8 = 7;
const TYPE_BYTE_SIZE_BITS: u8 = VAR_INT_ENCODING_BITS - TYPE_BITS;
const VAR_INT_CONTINUE_FLAG: u8 = 1 << VAR_INT_ENCODING_BITS;

/// Preserve the last bits of value binary
///
#[allow(unused)]
fn keep_bits(value: usize, bits: u8) -> usize {
    value & ((1 << bits) - 1)
}

/// Read the next N bytes from the reader
///
#[allow(unused)]
pub fn read_bytes<R: Read, const N: usize>(stream: &mut R) -> io::Result<[u8; N]> {
    let mut bytes = [0; N];
    stream.read_exact(&mut bytes)?;

    Ok(bytes)
}

/// Read a u32 from the reader
///
#[allow(unused)]
pub fn read_u32<R: Read>(stream: &mut R) -> io::Result<u32> {
    let bytes = read_bytes(stream)?;

    Ok(u32::from_be_bytes(bytes))
}

/// Read a hash from the reader
///
#[allow(unused)]
pub fn read_hash<R: Read>(stream: &mut R) -> io::Result<Hash> {
    let bytes = read_bytes(stream)?;

    Ok(Hash(bytes))
}

/// Read a vec until the delimiter is read
///
#[allow(unused)]
pub fn read_until_delimiter<R: Read>(stream: &mut R, delimiter: u8) -> io::Result<Vec<u8>> {
    let mut bytes = vec![];

    loop {
        let [byte] = read_bytes(stream)?;
        if byte == delimiter {
            break;
        }

        bytes.push(byte);
    }

    Ok(bytes)
}

/// Returns whether the first bit of u8 is 1 and returns the 7-bit truth value
///
#[allow(unused)]
pub fn read_var_int_byte<R: Read>(stream: &mut R) -> io::Result<(u8, bool)> {
    let [byte] = read_bytes(stream)?;
    let value = byte & !VAR_INT_CONTINUE_FLAG;
    let more_bytes = byte & VAR_INT_CONTINUE_FLAG != 0;

    Ok((value, more_bytes))
}

/// Read the type and size of the object
///
#[allow(unused)]
pub fn read_size_encoding<R: Read>(stream: &mut R) -> io::Result<usize> {
    let mut value = 0;
    let mut length = 0;

    loop {
        let (byte_value, more_bytes) = read_var_int_byte(stream).unwrap();
        value |= (byte_value as usize) << length;
        if !more_bytes {
            return Ok(value);
        }

        length += VAR_INT_ENCODING_BITS;
    }
}

///
///
#[allow(unused)]
pub fn write_size_encoding(number: usize) -> Vec<u8> {
    let mut num = vec![];
    let mut number = number;

    loop {
        if number >> VAR_INT_ENCODING_BITS > 0 {
            num.push((number & 0x7f) as u8 | 0x80);
        } else {
            num.push((number & 0x7f) as u8);
            break;
        }

        number >>= VAR_INT_ENCODING_BITS;
    }

    num
}

/// Read the first few fields of the object and parse
///
#[allow(unused)]
pub fn read_type_and_size<R: Read>(stream: &mut R) -> io::Result<(u8, usize)> {
    // Object type and uncompressed pack data size
    // are stored in a "size-encoding" variable-length integer.
    // Bits 4 through 6 store the type and the remaining bits store the size.
    let value = read_size_encoding(stream)?;
    let object_type = keep_bits(value >> TYPE_BYTE_SIZE_BITS, TYPE_BITS) as u8;
    let size = keep_bits(value, TYPE_BYTE_SIZE_BITS)
        | (value >> VAR_INT_ENCODING_BITS << TYPE_BYTE_SIZE_BITS);

    Ok((object_type, size))
}

/// The offset for an OffsetDelta object
///
#[allow(unused)]
pub fn read_offset_encoding<R: Read>(stream: &mut R) -> io::Result<u64> {
    // Like the object length, the offset for an OffsetDelta object
    // is stored in a variable number of bytes,
    // with the most significant bit of each byte indicating whether more bytes follow.
    // However, the object length encoding allows redundant values,
    // e.g. the 7-bit value [n] is the same as the 14- or 21-bit values [n, 0] or [n, 0, 0].
    // Instead, the offset encoding adds 1 to the value of each byte except the least significant one.
    // And just for kicks, the bytes are ordered from *most* to *least* significant.
    let mut value = 0;
    loop {
        let (byte_value, more_bytes) = read_var_int_byte(stream)?;

        value = (value << VAR_INT_ENCODING_BITS) | byte_value as u64;
        if !more_bytes {
            return Ok(value);
        }

        value += 1;
    }
}

///
/// # Example
///
/// ```
/// let ns :u64 = 0x4af;
/// let re = write_offset_encoding(ns);
/// println!("{:?}",re);
/// ```
///
#[allow(unused)]
pub fn write_offset_encoding(number: u64) -> Vec<u8> {
    let mut num = vec![];
    let mut number = number;

    num.push((number & 0x7f) as u8);
    number >>= 7;

    while number > 0 {
        num.push(((number & 0x7f) - 1) as u8 | 0x80);
        number >>= 7;
    }

    num.reverse();

    num
}

///
///
///
#[allow(unused)]
pub fn read_partial_int<R: Read>(
    stream: &mut R,
    bytes: u8,
    present_bytes: &mut u8,
) -> io::Result<usize> {
    let mut value: usize = 0;
    for byte_index in 0..bytes {
        if *present_bytes & 1 != 0 {
            let [byte] = read_bytes(stream)?;
            value |= (byte as usize) << (byte_index * 8);
        }
        *present_bytes >>= 1;
    }
    Ok(value)
}

///
///
#[allow(unused)]
pub fn seek(file: &mut File, offset: u64) -> io::Result<()> {
    file.seek(SeekFrom::Start(offset))?;

    Ok(())
}

///
///
#[allow(unused)]
pub fn get_offset(file: &mut File) -> io::Result<u64> {
    file.stream_position()
}

/// Call reader() to process a zlib stream from a file.
/// Reset the file offset afterwards to the end of the zlib stream,
/// since ZlibDecoder uses BufReader, which may consume extra bytes.
#[allow(unused)]
pub fn read_zlib_stream_exact<T, F>(file: &mut File, reader: F) -> Result<T, GitError>
where
    F: FnOnce(&mut ZlibDecoder<&mut File>) -> Result<T, GitError>,
{
    let offset = get_offset(file).unwrap();
    let mut decompressed = ZlibDecoder::new(file);
    let result = reader(&mut decompressed);
    let zlib_end = offset + decompressed.total_in();

    seek(decompressed.into_inner(), zlib_end).unwrap();

    result
}

///
///
///
#[allow(unused)]
pub fn u32_vec(value: u32) -> Vec<u8> {
    let mut result: Vec<u8> = vec![];
    result.push((value >> 24 & 0xff) as u8);
    result.push((value >> 16 & 0xff) as u8);
    result.push((value >> 8 & 0xff) as u8);
    result.push((value & 0xff) as u8);
    result
}

///
///
///
#[allow(unused)]
pub fn get_pack_raw_data(data: Vec<u8>) -> Vec<u8> {
    let result = &data[12..data.len() - 20];
    let result = result.to_vec();
    result
}

///
///
///
#[allow(unused)]
fn get_hash_form_filename(filename: &str) -> String {
    String::from(&filename[5..45])
}

/// Return a list of pack files in the pack directory.
#[allow(unused)]
pub fn find_all_pack_file(pack_dir: &str) -> (Vec<PathBuf>, Vec<Hash>) {
    let mut file_path = vec![];
    let mut hash_list = vec![];
    let mut object_root = std::path::PathBuf::from(pack_dir);

    let paths = std::fs::read_dir(&object_root).unwrap();

    for path in paths {
        if let Ok(pack_file) = path {
            let _file_name = pack_file.file_name();
            let _file_name = _file_name.to_str().unwrap();

            // only find the .pack file, and find the .idx file
            if &_file_name[_file_name.len() - 4..] == "pack" {
                let hash_string = get_hash_form_filename(&_file_name);
                let _hash = Hash::from_str(&hash_string).unwrap();
                hash_list.push(_hash);

                object_root.push(&_file_name.to_string());
                file_path.push(object_root.clone());
                object_root.pop();
            }
        }
    }

    (file_path, hash_list)
}

#[cfg(test)]
mod test {}
