//!
//!
//!

use anyhow::Result;

mod gateway;
mod database;
mod lfs;
mod utils;
mod errors;
mod git;
mod gust;

use gateway::api::lib;

fn main() -> Result<()> {
    lib::main().unwrap();

    Ok(())
}
