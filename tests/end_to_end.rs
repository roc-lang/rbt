use assert_cmd::cmd::Command;
use std::time::Duration;
use tempfile::TempDir;

#[test]
fn test_file_inputs() {
    let root = TempDir::new().unwrap();

    Command::new("roc")
        .arg("run")
        .arg("--linker=legacy")
        .arg("rbt.roc")
        .arg("--")
        .arg("--root-dir")
        .arg(root.path().display().to_string())
        .current_dir("tests/end_to_end/file_inputs")
        .timeout(Duration::from_secs(10))
        .assert()
        .success();

    let store_path = root
        .path()
        .join("store")
        .read_dir()
        .expect("could not read dir")
        .flatten()
        .filter(|item| {
            item.file_type()
                .map(|ftype| ftype.is_dir())
                .unwrap_or(false)
        })
        .map(|item| item.path())
        .next()
        .expect("expected at least one directory in the store path");

    let greeting = std::fs::read_to_string(store_path.join("out")).unwrap();

    assert_eq!(String::from("Hello, World!\n"), greeting)
}

#[test]
fn test_env() {
    let root = TempDir::new().unwrap();

    Command::new("roc")
        .arg("run")
        .arg("--linker=legacy")
        .arg("rbt.roc")
        .arg("--")
        .arg("--root-dir")
        .arg(root.path().display().to_string())
        .current_dir("tests/end_to_end/env")
        .timeout(Duration::from_secs(10))
        .assert()
        .success();

    let store_path = root
        .path()
        .join("store")
        .read_dir()
        .expect("could not read dir")
        .flatten()
        .filter(|item| {
            item.file_type()
                .map(|ftype| ftype.is_dir())
                .unwrap_or(false)
        })
        .map(|item| item.path())
        .next()
        .expect("expected at least one directory in the store path");

    let greeting = std::fs::read_to_string(store_path.join("out")).unwrap();

    assert_eq!(String::from("Hello, World!\n"), greeting)
}

#[test]
fn test_job_inputs() {
    let root = TempDir::new().unwrap();

    Command::new("roc")
        .arg("run")
        .arg("--linker=legacy")
        .arg("rbt.roc")
        .arg("--")
        .arg("--root-dir")
        .arg(root.path().display().to_string())
        .current_dir("tests/end_to_end/job_inputs")
        .timeout(Duration::from_secs(10))
        .assert()
        .success();

    let store_path = root
        .path()
        .join("store")
        .read_dir()
        .expect("could not read dir")
        .flatten()
        .filter(|item| {
            item.file_type()
                .map(|ftype| ftype.is_dir())
                .unwrap_or(false)
        })
        .map(|item| item.path())
        .next()
        .expect("expected at least one directory in the store path");

    let greeting = std::fs::read_to_string(store_path.join("out")).unwrap();

    assert_eq!(String::from("Hello, World!\n"), greeting)
}
