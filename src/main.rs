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
    /// The pixi environment to inject packages into.
    #[arg(short, long)]
    environment: Option<String>,

    /// The prefix to inject packages into.
    #[arg(short, long)]
    prefix: Option<PathBuf>,

    /// The package to inject into the environment.
    #[arg(long)]
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

    let target_prefix = match (cli.prefix, cli.environment) {
        (Some(_), Some(_)) => {
            return Err(anyhow::anyhow!(
                "Both --prefix and --environment cannot be provided at the same time."
            ));
        }
        (Some(prefix), None) => prefix,
        (None, Some(environment)) => PathBuf::from(format!(".pixi/envs/{}", environment)),
        (None, None) => PathBuf::from(".pixi/envs/default"),
    };
    tracing::info!("Using target prefix: {:?}", target_prefix);

    let packages = cli.package;

    pixi_inject::pixi_inject(target_prefix, packages).await
}
