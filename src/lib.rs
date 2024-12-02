use anyhow::Result;
use rattler::install::{link_package, InstallDriver, InstallOptions, PythonInfo};
use rattler_conda_types::{prefix_record::PathsEntry, PackageRecord, PrefixRecord, RepoDataRecord};
use rattler_package_streaming::fs::extract;
use std::{
    fmt::{Display, Formatter},
    path::{Path, PathBuf},
};

use std::collections::HashSet;

use anyhow::Context;
use rattler_conda_types::{package::ArchiveType, Platform};
use rattler_index::{package_record_from_conda, package_record_from_tar_bz2};
use reqwest::Url;

pub struct PackageRecordVec(pub Vec<PackageRecord>);

impl Display for PackageRecordVec {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[{}]",
            self.0
                .iter()
                .map(|p| format!("{}", p))
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}

pub async fn pixi_inject(target_prefix: PathBuf, packages: Vec<PathBuf>) -> Result<()> {
    if packages.is_empty() {
        return Err(anyhow::anyhow!("No packages were provided."));
    }

    let installed_packages = PrefixRecord::collect_from_prefix(&target_prefix)?;
    let installed_package_records = installed_packages
        .iter()
        .map(|e| e.repodata_record.clone().package_record)
        .collect::<Vec<_>>();

    let injected_packages = packages
        .iter()
        .map(|p| {
            let record = package_record_from_archive(p)?;
            anyhow::Ok((p.clone(), record))
        })
        .collect::<Result<Vec<_>>>()?;

    tracing::debug!(
        "Installed packages: {}",
        PackageRecordVec(
            installed_packages
                .iter()
                .map(|p| p.repodata_record.package_record.clone())
                .collect::<Vec<_>>()
        )
    );
    tracing::debug!(
        "Injected packages: {}",
        PackageRecordVec(
            injected_packages
                .iter()
                .map(|p| p.1.clone())
                .collect::<Vec<_>>()
        )
    );

    let not_matching_platform = injected_packages
        .iter()
        .map(|p| &p.1)
        .filter(|p| {
            p.subdir != Platform::NoArch.to_string() && p.subdir != Platform::current().to_string()
        })
        .collect::<Vec<_>>();
    if !not_matching_platform.is_empty() {
        return Err(anyhow::anyhow!(
            "Packages with platform not matching the current platform ({}) were found: {}",
            Platform::current().to_string(),
            not_matching_platform
                .into_iter()
                .map(|p| format!("{} ({})", p, p.subdir.clone()))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }

    tracing::debug!("Validating package compatibility with prefix.");
    let all_records = installed_package_records
        .iter()
        .chain(injected_packages.iter().map(|p| &p.1))
        .collect();
    PackageRecord::validate(all_records)?;
    tracing::debug!("All packages are compatible with the prefix.");

    // check whether the package is already installed
    let injected_package_names = injected_packages
        .iter()
        .map(|p| p.1.name.as_normalized())
        .collect::<HashSet<_>>();
    let installed_package_names = installed_packages
        .iter()
        .map(|p| p.repodata_record.package_record.name.as_normalized())
        .collect::<HashSet<_>>();
    if !injected_package_names.is_disjoint(&installed_package_names) {
        return Err(anyhow::anyhow!(
            "Some of the packages are already installed: {}",
            injected_package_names
                .intersection(&injected_package_names)
                .map(|p| p.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }

    eprintln!(
        "⏳ Extracting and installing {} package{} to {}...",
        packages.len(),
        if packages.len() == 1 { "" } else { "s" },
        target_prefix.display()
    );

    let driver = InstallDriver::default();
    let python_info = if installed_package_names.contains("python") {
        Some(PythonInfo::from_python_record(
            &installed_packages
                .iter()
                .find(|&p| p.repodata_record.package_record.name.as_normalized() == "python")
                .context("Could not find python package in installed packages")?
                .repodata_record
                .package_record
                .clone(),
            Platform::current(),
        )?)
    } else {
        None
    };
    let options = InstallOptions {
        python_info,
        ..Default::default()
    };

    for (path, package_record) in injected_packages.iter() {
        let repodata_record = RepoDataRecord {
            package_record: package_record.clone(),
            file_name: path
                .to_str()
                .context("Could not create file name from path")?
                .to_string(),
            url: Url::from_file_path(path.canonicalize()?)
                .map_err(|_| anyhow::anyhow!("Could not convert path to URL"))?,
            channel: "".to_string(),
        };
        install_package_to_environment_from_archive(
            target_prefix.as_path(),
            path.clone(),
            repodata_record,
            &driver,
            &options,
        )
        .await?;
        tracing::debug!("Installed package: {}", path.display());
    }

    eprintln!("✅ Finished installing packages to prefix.");

    tracing::debug!("Finished running pixi-inject");
    Ok(())
}

fn package_record_from_archive(file: &Path) -> Result<PackageRecord> {
    let archive_type = ArchiveType::split_str(file.to_string_lossy().as_ref())
        .context("Could not create ArchiveType")?
        .1;
    match archive_type {
        ArchiveType::TarBz2 => package_record_from_tar_bz2(file),
        ArchiveType::Conda => package_record_from_conda(file),
    }
    .map_err(|e| anyhow::anyhow!("Could not read package record from archive: {}", e))
}

/// Install a package into the environment and write a `conda-meta` file that
/// contains information about how the file was linked.
async fn install_package_to_environment_from_archive(
    target_prefix: &Path,
    package_path: PathBuf,
    repodata_record: RepoDataRecord,
    install_driver: &InstallDriver,
    install_options: &InstallOptions,
) -> anyhow::Result<()> {
    // Link the contents of the package into our environment. This returns all the
    // paths that were linked.
    let paths = link_package_from_archive(
        &package_path,
        target_prefix,
        install_driver,
        install_options.clone(),
    )
    .await?;

    // Construct a PrefixRecord for the package
    let prefix_record = PrefixRecord {
        repodata_record,
        package_tarball_full_path: None,
        extracted_package_dir: None,
        files: paths
            .iter()
            .map(|entry| entry.relative_path.clone())
            .collect(),
        paths_data: paths.into(),
        requested_spec: None,
        link: None,
    };

    // Create the conda-meta directory if it doesnt exist yet.
    let target_prefix = target_prefix.to_path_buf();
    let result = tokio::task::spawn_blocking(move || {
        let conda_meta_path = target_prefix.join("conda-meta");
        std::fs::create_dir_all(&conda_meta_path)?;

        // Write the conda-meta information
        let pkg_meta_path = conda_meta_path.join(prefix_record.file_name());
        prefix_record.write_to_path(pkg_meta_path, true)
    })
    .await;
    match result {
        Ok(result) => Ok(result?),
        Err(err) => {
            if let Ok(panic) = err.try_into_panic() {
                std::panic::resume_unwind(panic);
            }
            // The operation has been cancelled, so we can also just ignore everything.
            Ok(())
        }
    }
}

// https://github.com/conda/rattler/pull/937
async fn link_package_from_archive(
    package_path: &Path,
    target_dir: &Path,
    driver: &InstallDriver,
    options: InstallOptions,
) -> Result<Vec<PathsEntry>> {
    let temp_dir = tempfile::tempdir()?;

    tracing::debug!(
        "extracting {} to temporary directory {}",
        package_path.display(),
        temp_dir.path().display()
    );
    extract(package_path, temp_dir.path())?;
    link_package(temp_dir.path(), target_dir, driver, options)
        .await
        .map_err(|e| anyhow::anyhow!("Could not create temporary directory: {}", e))
}
