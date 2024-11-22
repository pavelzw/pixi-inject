use std::path::PathBuf;

use clap::Parser;
use clap_verbosity_flag::Verbosity;

use anyhow::Result;
use tracing_log::AsTrace;

/* -------------------------------------------- CLI -------------------------------------------- */

/// The pixi-inject CLI.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    #[arg(long)]
    prefix: PathBuf,

    #[arg(short, long)]
    package: Vec<PathBuf>,

    #[command(flatten)]
    verbose: Verbosity,
}

/* -------------------------------------------- MAIN ------------------------------------------- */

/// The main entrypoint for the pixi-inject CLI.
#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    tracing_subscriber::FmtSubscriber::builder()
        .with_max_level(cli.verbose.log_level_filter().as_trace())
        .init();

    tracing::debug!("Starting pixi-inject CLI");
    tracing::debug!("Parsed CLI options: {:?}", cli);

    let target_prefix = cli.prefix;
    let packages = cli.package;

    pixi_inject::pixi_inject(target_prefix, packages).await
}
