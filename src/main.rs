//!
//! # Gust - A Monorepo Platform for Git
//!
//! Google has a monolithic repository platform, Piper, with more than 100 TB of data. It's building
//! on top of Google's infrastructure. Gust's purpose is to imitate Piper's architecture to
//! implement a developing platform which compatible Git and trunk-based development flow for
//! collaboration, open source compliance and supply chain management and DevSecOps.
//!
//! ## Git Compatible
//!
//! Git is a content-addressable filesystem and a distributed collaboration system. All files of
//! a single repository persisted on the disk of the machine. It brings a lot of benefits to
//! performance and maintenance. But it also has challenges for monorepo. It is hard to manage a
//! vast code repository like a repo has 20 TB, which is typical in a middle size enterprise.
//!
//! ## Trunk-based Development
//!
//!
//! ## Collaboration
//!
//!
//! ## Open Source Compliance
//!
//!
//! ## Supply Chain Management
//!
//!
//! ## References
//!
//! 1. [What is monorepo? (and should you use it?)](https://semaphoreci.com/blog/what-is-monorepo)
//! 2. [Monorepo: A single repository for all your code](https://medium.com/@mattklein123/monorepo-a-single-repository-for-all-your-code-86a852bff054)
//! 3. [Why Google Stores Billions of Lines of Code in a Single Repository](https://cacm.acm.org/magazines/2016/7/204032-why-google-stores-billions-of-lines-of-code-in-a-single-repository)
//! 4. [Trunk Based Development](https://trunkbaseddevelopment.com)


pub mod gateway;
pub mod database;
pub mod utils;
pub mod errors;
pub mod git;
pub mod gust;

use anyhow::Result;

use crate::gateway::api::lib;

/// The main entry of the application.
///
/// ### TODO
/// 1. Add `clap` to parse command line arguments, don't start gateway service directly in the main function.
pub fn main() -> Result<()> {
    lib::main().unwrap();

    Ok(())
}
