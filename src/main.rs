use clap::Parser;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = diagram::cli::Cli::parse();
    cli.run().await
}
