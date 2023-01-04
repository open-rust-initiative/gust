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

use anyhow::Result;
use gateway::api::lib;

fn main() -> Result<()> {
    lib::main().unwrap();
    Ok(())
}
