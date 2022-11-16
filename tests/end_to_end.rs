use anyhow::{Context, Result};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

fn output_of_default_job(root: &TempDir, rbt_dot_roc: &Path) -> Result<PathBuf> {
    let current_dir = rbt_dot_roc
        .parent()
        .context("failed to get parent of the target rbt.roc")?;
    let filename = rbt_dot_roc
        .file_name()
        .context("failed to get the filename of the target rbt.roc")?;

    let output = std::process::Command::new("roc")
        .arg("run")
        .arg(filename)
        .arg("--")
        .arg("--root-dir")
        .arg(root.path().display().to_string())
        .arg("--print-root-output-paths")
        .current_dir(current_dir.display().to_string())
        .output()
        .context("failed to spawn `roc run`")?;

    if !output.status.success() {
        anyhow::bail!("failed to `roc run`: {:#?}", output);
    }

    Ok(PathBuf::from(
        std::str::from_utf8(&output.stdout)
            .context("could not convert output to a UTF-8 string")?
            .trim(),
    ))
}

#[test]
fn test_file_inputs() {
    let root = TempDir::new().unwrap();

    let store_path = output_of_default_job(
        &root,
        &PathBuf::from("tests/end_to_end/file_inputs/rbt.roc"),
    )
    .unwrap();

    let greeting = std::fs::read_to_string(store_path.join("out")).unwrap();

    assert_eq!(String::from("Hello, World!\n"), greeting)
}

#[test]
fn test_file_inputs_renaming() {
    let root = TempDir::new().unwrap();

    let store_path = output_of_default_job(
        &root,
        &PathBuf::from("tests/end_to_end/file_inputs_renaming/rbt.roc"),
    )
    .unwrap();

    let greeting = std::fs::read_to_string(store_path.join("out")).unwrap();

    assert_eq!(String::from("Hello, World!\n"), greeting)
}

#[test]
fn test_env() {
    let root = TempDir::new().unwrap();

    let store_path =
        output_of_default_job(&root, &PathBuf::from("tests/end_to_end/env/rbt.roc")).unwrap();

    let greeting = std::fs::read_to_string(store_path.join("out")).unwrap();

    assert_eq!(String::from("Hello, World!\n"), greeting)
}

#[test]
fn test_cleanup() {
    let root = TempDir::new().unwrap();

    let store_path = output_of_default_job(
        &root,
        &PathBuf::from("tests/end_to_end/env_cleanup/rbt.roc"),
    )
    .unwrap();

    let echoed_variables = std::fs::read_to_string(store_path.join("out")).unwrap();

    let variables: HashSet<&str> = echoed_variables
        .split_ascii_whitespace()
        .map(|str| {
            *str.split_terminator('=')
                .collect::<Vec<&str>>()
                .first()
                .unwrap()
        })
        .collect();

    // This are built-ins environment variables from the shell, and are generated every time we run a command
    // Once we add support to save a command's stdout instead of using a shell for everything, this can be removed from the tests
    let expected_variables = HashSet::from(["_", "PWD", "SHLVL", "HOME"]);

    assert_eq!(expected_variables, variables)
}

#[test]
fn test_job_inputs() {
    let root = TempDir::new().unwrap();

    let store_path =
        output_of_default_job(&root, &PathBuf::from("tests/end_to_end/job_inputs/rbt.roc"))
            .unwrap();

    let greeting = std::fs::read_to_string(store_path.join("out")).unwrap();

    assert_eq!(String::from("Hello, World!\n"), greeting)
}

#[test]
fn test_job_inputs_branching() {
    let root = TempDir::new().unwrap();

    let store_path = output_of_default_job(
        &root,
        &PathBuf::from("tests/end_to_end/job_inputs_branching/rbt.roc"),
    )
    .unwrap();

    let greeting = std::fs::read_to_string(store_path.join("out")).unwrap();

    assert_eq!(String::from("Hello, World!\n"), greeting)
}
