//!
//!
//!

use anyhow::Result;

mod gateway;
mod database;
mod gust;
mod lfs;
mod utils;
mod errors;
mod git;

use gateway::api::lib;

fn main() -> Result<()> {
    lib::main().unwrap();
    Ok(())
}
