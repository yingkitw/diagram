mod cli;
mod diagram;
mod layout;
mod mcp;
mod parser;
mod renderer;

use clap::Parser;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = cli::Cli::parse();
    cli.run().await
}
