use std::{path::PathBuf, process::Command};

use rattler_conda_types::Platform;
use rstest::*;
use tempfile::tempdir;

struct Options {
    prefix: PathBuf,
    package: Vec<PathBuf>,
    _output_dir: tempfile::TempDir,
}

#[fixture]
fn options(#[default("simple-example")] test_case: String) -> Options {
    let output_dir = tempdir().unwrap();

    // copy pixi.toml and pixi.lock to temporary location
    let pixi_toml = output_dir.path().join("pixi.toml");
    let pixi_lock = output_dir.path().join("pixi.lock");
    std::fs::copy(
        format!("tests/resources/{}/pixi.toml", test_case),
        &pixi_toml,
    )
    .unwrap();
    std::fs::copy(
        format!("tests/resources/{}/pixi.lock", test_case),
        &pixi_lock,
    )
    .unwrap();

    let pixi_install = Command::new("pixi")
        .arg("install")
        .arg("--manifest-path")
        .arg(pixi_toml)
        .output()
        .unwrap();
    assert!(pixi_install.status.success());

    let prefix = output_dir.path().join(".pixi").join("envs").join("default");

    let package = match Platform::current() {
        Platform::Linux64 => "linux-64-pydantic-core-2.26.0-py313h920b4c0_0.conda",
        Platform::LinuxAarch64 => "linux-aarch64-pydantic-core-2.26.0-py313h8aa417a_0.conda",
        Platform::OsxArm64 => "osx-arm64-pydantic-core-2.26.0-py313hdde674f_0.conda",
        Platform::Osx64 => "osx-64-pydantic-core-2.26.0-py313h3c055b9_0.conda",
        Platform::Win64 => "win-64-pydantic-core-2.26.0-py313hf3b5b86_0.conda",
        _ => panic!("Unsupported platform"),
    };
    let package = PathBuf::from(format!("tests/resources/packages/{}", package));
    assert!(package.exists());
    Options {
        prefix,
        package: vec![package],
        _output_dir: output_dir,
    }
}

#[fixture]
fn required_fs_objects() -> Vec<&'static str> {
    let pydantic_core_conda_meta = match Platform::current() {
        Platform::Linux64 => "conda-meta/pydantic-core-2.26.0-py313h920b4c0_0.json",
        Platform::LinuxAarch64 => "conda-meta/pydantic-core-2.26.0-py313h8aa417a_0.json",
        Platform::OsxArm64 => "conda-meta/pydantic-core-2.26.0-py313hdde674f_0.json",
        Platform::Osx64 => "conda-meta/pydantic-core-2.26.0-py313h3c055b9_0.json",
        Platform::Win64 => "conda-meta/pydantic-core-2.26.0-py313hf3b5b86_0.json",
        _ => panic!("Unsupported platform"),
    };

    vec![
        #[cfg(not(unix))]
        "Lib\\site-packages\\pydantic_core",
        #[cfg(unix)]
        "lib/python3.13/site-packages/pydantic_core",
        pydantic_core_conda_meta,
    ]
}

#[rstest]
#[tokio::test]
async fn test_noarch_python(options: Options) {
    pixi_inject::pixi_inject(
        options.prefix.clone(),
        vec!["tests/resources/packages/setuptools-75.6.0-pyhff2d567_1.conda".into()],
    )
    .await
    .unwrap();

    #[cfg(not(unix))]
    let required_fs_objects = vec![
        "Lib\\site-packages\\setuptools\\__init__.py",
        "Lib\\site-packages\\pkg_resources\\__init__.py",
    ];
    #[cfg(unix)]
    let required_fs_objects = vec![
        "lib/python3.13/site-packages/setuptools/__init__.py",
        "lib/python3.13/site-packages/pkg_resources/__init__.py",
    ];
    for fs_object in required_fs_objects {
        assert!(options.prefix.join(fs_object).exists());
    }
}

#[rstest]
#[tokio::test]
async fn test_simple_example(options: Options, required_fs_objects: Vec<&'static str>) {
    pixi_inject::pixi_inject(options.prefix.clone(), options.package)
        .await
        .unwrap();

    for fs_object in required_fs_objects {
        assert!(options.prefix.join(fs_object).exists());
    }
}

#[rstest]
#[tokio::test]
async fn test_install_twice(options: Options) {
    pixi_inject::pixi_inject(options.prefix.clone(), options.package.clone())
        .await
        .unwrap();

    let result = pixi_inject::pixi_inject(options.prefix.clone(), options.package).await;
    assert!(result.is_err());
    assert!(result
        .err()
        .unwrap()
        .to_string()
        .contains("Some of the packages are already installed: pydantic-core"))
}

#[rstest]
#[case("already-installed-different-version".to_string())]
#[case("already-installed".to_string())]
#[tokio::test]
async fn test_already_installed(
    #[case] _test_case: String,
    #[with(_test_case.clone())] options: Options,
) {
    let result = pixi_inject::pixi_inject(options.prefix.clone(), options.package).await;
    assert!(result.is_err());
    assert!(result
        .err()
        .unwrap()
        .to_string()
        .contains("Some of the packages are already installed: pydantic-core"))
}
