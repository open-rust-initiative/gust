//!
//! # Gust - A Monorepo Platform for Git
//!
//! Google has a monorepo system, __Piper__, with more than 100 TB of data. It's building
//! on top of Google's infrastructure. Gust's purpose is to imitate Piper's architecture to
//! implement a developing a monorepo platform which compatible Git and trunk-based development flow for
//! collaboration, open source compliance and supply chain management and DevSecOps.
//!
//! ## Git Compatible
//!
//! Git is a content-addressable file system. It is also a distributed collaboration system. All of
//! the files in a single repository are persisted on the machine's hard drive. This has many
//! advantages for performance and maintenance. But it also presents challenges for monorepo. It
//! isn't easy to manage a large code repository, such as a 20TB repo, which is typical in a
//! medium-sized company.
//!
//! Git is the world's most widely used version control system, and Gust aims to build a bridge
//! between Git and Monorepo. Git can 'clone' or 'pull' any folder from Monorepo into the local
//! development environment as a Git repository and 'push' it back. Gust hosts a codebase of
//! monorepo with distribution databases such as SQL, NoSQL, and Graph Database.
//!
//! ## Trunk-based Development
//!
//! ## References
//!
//! 1. [What is monorepo? (and should you use it?)](https://semaphoreci.com/blog/what-is-monorepo)
//! 2. [Monorepo: A single repository for all your code](https://medium.com/@mattklein123/monorepo-a-single-repository-for-all-your-code-86a852bff054)
//! 3. [Why Google Stores Billions of Lines of Code in a Single Repository](https://cacm.acm.org/magazines/2016/7/204032-why-google-stores-billions-of-lines-of-code-in-a-single-repository)
//! 4. [Trunk Based Development](https://trunkbaseddevelopment.com)
//! 5. [Branching strategies: Git-flow vs trunk-based development](https://www.devbridge.com/articles/branching-strategies-git-flow-vs-trunk-based-development/)
//! 6. [Monorepo.tools](https://monorepo.tools)
//! 7. [Google Open Source Third Party](https://opensource.google/documentation/reference/thirdparty)

pub mod errors;
pub mod gateway;
pub mod git;
pub mod gust;
pub mod utils;

use std::path::PathBuf;

use anyhow::Result;
use clap::{command, Parser};
use git::protocol::ServeCommand;

use crate::gateway::api::lib;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    // custom configuration file
    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,

    //custom log file
    #[arg(short, long, value_name = "FILE")]
    log_path: Option<PathBuf>,

    //subcommand serve
    #[command(subcommand)]
    serve_command: Option<ServeCommand>,
}

/// The main entry of the application.
///
/// ### TODO
/// 1. Add `clap` to parse command line arguments, don't start gateway service directly in the main function.
/// 2. Add `log` function and initialization to log the application's running status.
/// 3. Add `config` function to load the configuration file when the application running.
pub fn main() -> Result<()> {
    // let cli = Cli::parse();
    lib::main().unwrap();

    // tracing_subscriber::fmt()
    // .with_max_level(tracing::Level::DEBUG)
    // .with_test_writer()
    // .init();

    Ok(())
}
