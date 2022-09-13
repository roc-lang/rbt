use assert_cmd::cmd::Command;
use std::time::Duration;
use tempfile::TempDir;

#[test]
fn test_hello_world() {
    let root = TempDir::new().unwrap();

    Command::new("roc")
        .arg("build")
        .arg("--linker=legacy")
        .arg("examples/hello/rbt.roc")
        .assert()
        .success();

    Command::new("./build")
        .arg("--root-dir")
        .arg(root.path().display().to_string())
        .current_dir("examples/hello")
        .timeout(Duration::from_secs(10))
        .assert()
        .success();

    let store_path = root
        .path()
        .join("store")
        .read_dir()
        .expect("`store` under the root should be a directory")
        .next()
        .expect("there should be only one store path for the hello example")
        .expect("could not read the dir entry")
        .path();

    let greeting = std::fs::read_to_string(store_path.join("out")).unwrap();

    assert_eq!(String::from("Hello, World!\n"), greeting)
}
