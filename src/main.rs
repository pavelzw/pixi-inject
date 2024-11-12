use std::path::PathBuf;

use clap::Parser;
use clap_verbosity_flag::Verbosity;

use anyhow::Result;
use rattler_conda_types::{package::ArchiveType, PackageRecord, PrefixRecord};
use rattler_index::{package_record_from_conda, package_record_from_tar_bz2};
use tracing_log::AsTrace;

/* -------------------------------------------- CLI -------------------------------------------- */

/// The pixi-inject CLI.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    #[arg(long)]
    prefix: Option<PathBuf>,

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

    let prefix = cli.prefix.unwrap(); // todo: fix unwrap
    let packages = cli.package;

    if packages.len() == 0 {
        return Err(anyhow::anyhow!("No packages were provided."));
    }

    let prefix_package_records = PrefixRecord::collect_from_prefix(&prefix)?
        .iter()
        .map(|e| e.repodata_record.clone().package_record)
        .collect::<Vec<_>>();

    let injected_packages: Vec<(PathBuf, ArchiveType)> = packages
        .iter()
        .filter_map(|e| {
            ArchiveType::split_str(e.as_path().to_string_lossy().as_ref())
                .map(|(p, t)| (PathBuf::from(format!("{}{}", p, t.extension())), t))
        })
        .collect();

    let mut package_records = Vec::new();

    tracing::info!("Retrieving metadata of {} injected packages.", injected_packages.len());
    for (path, archive_type) in injected_packages.iter() {
        let package_record = match archive_type {
            ArchiveType::TarBz2 => package_record_from_tar_bz2(path),
            ArchiveType::Conda => package_record_from_conda(path),
        }?;
        package_records.push(package_record);
    }

    tracing::debug!("Validating package compatibility with prefix.");
    let all_records = prefix_package_records.iter().chain(package_records.iter()).collect();
    PackageRecord::validate(all_records)?;
    tracing::debug!("All packages are compatible with the prefix.");

    tracing::debug!("Finished running pixi-inject");
    Ok(())
}
