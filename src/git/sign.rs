//!
//!
//!
//!
//!
//!
//!

use std::fmt::Display;

use bstr::ByteSlice;

use crate::errors::GitError;

///
#[allow(unused)]
#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone)]
pub struct AuthorSign {
    pub t: String,
    pub name: String,
    pub email: String,
    pub timestamp: usize,
    pub timezone: String,
}

///
impl Display for AuthorSign {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{} <{}> {} {}", self.name, self.email, self.timestamp, self.timezone)
    }
}

///
impl AuthorSign {
    ///
    #[allow(unused)]
    pub(crate) fn decode_from_data(&mut self, data: Vec<u8>) -> Result<(), GitError> {
        let mut data = data;

        let name_start = data.find_byte(0x20).unwrap();

        self.t = String::from_utf8(data[..name_start].to_vec()).unwrap();

        let email_start = data.find_byte(0x3C).unwrap();
        let email_end = data.find_byte(0x3E).unwrap();

        self.name = data[name_start + 1..email_start - 1].to_str().unwrap().to_string();
        self.email = data[email_start + 1..email_end].to_str().unwrap().to_string();
        data = data[email_end + 2..].to_vec();

        let timestamp_split = data.find_byte(0x20).unwrap();
        self.timestamp = data[0..timestamp_split].to_str().unwrap().parse::<usize>().unwrap();
        self.timezone = data[timestamp_split + 1..].to_str().unwrap().to_string();

        Ok(())
    }

    ///
    #[allow(unused)]
    pub(crate) fn encode_to_data(&self) -> Result<Vec<u8>, GitError> {
        let mut data = Vec::new();

        data.extend_from_slice(self.t.as_bytes());
        data.extend_from_slice(0x20u8.to_be_bytes().as_ref());
        data.extend_from_slice(self.name.as_bytes());
        data.extend_from_slice(0x20u8.to_be_bytes().as_ref());
        data.extend_from_slice(0x3Cu8.to_be_bytes().as_ref());
        data.extend_from_slice(self.email.as_bytes());
        data.extend_from_slice(0x3Eu8.to_be_bytes().as_ref());
        data.extend_from_slice(0x20u8.to_be_bytes().as_ref());
        data.extend_from_slice(self.timestamp.to_string().as_bytes());
        data.extend_from_slice(0x20u8.to_be_bytes().as_ref());
        data.extend_from_slice(self.timezone.as_bytes());

        Ok(data)
    }
}