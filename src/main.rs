//!
//!
//!
extern crate core;

mod git;
mod gateway;
mod database;
mod gust;
mod lfs;
mod utils;
mod errors;

use anyhow::Result;

use crate::git::Type;

fn main() -> Result<()> {
    println!("{:?}", Type::Tree.to_string());

    Ok(())
}
