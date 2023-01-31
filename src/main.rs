use clap::Parser;

use crate::entry::{entry, Cli};

pub mod entry;
pub mod init;
pub mod linter;
pub mod make_migrations;
pub mod merge_migrations;
pub mod migrate;
pub mod squash_migrations;
pub mod utils;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli: Cli = Cli::parse();

    entry(cli).await
}
