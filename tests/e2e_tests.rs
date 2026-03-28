use assert_cmd::Command;
use assert_fs::prelude::*;
use predicates::prelude::*;
use std::fs;
use std::time::Duration;

const BIN_NAME: &str = "memocp";

#[test]
fn test_help() {
    get_cmd()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage:"));
}

#[test]
fn test_version() {
    get_cmd()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::is_match("memocp \\d+\\.\\d+\\.\\d+").unwrap());
}

#[test]
fn test_copy() {
    let source = assert_fs::TempDir::new().unwrap();
    let destination = assert_fs::TempDir::new().unwrap();
    let state_file = assert_fs::NamedTempFile::new("memocp.db").unwrap();

    source
        .child("file1.txt")
        .write_str("Hello, world!")
        .unwrap();
    source
        .child("subdir/file2.txt")
        .write_str("Nested content")
        .unwrap();

    get_cmd()
        .current_dir(source.path())
        .arg(source.path())
        .arg(destination.path())
        .arg("--state-file")
        .arg(state_file.path())
        .assert()
        .success();

    destination
        .child("file1.txt")
        .assert(predicate::path::exists());
    destination
        .child("file1.txt")
        .assert(predicate::str::diff("Hello, world!"));
    destination
        .child("subdir/file2.txt")
        .assert(predicate::path::exists());
    destination
        .child("subdir/file2.txt")
        .assert(predicate::str::diff("Nested content"));
}

#[test]
fn test_glob() {
    let source = assert_fs::TempDir::new().unwrap();
    let destination = assert_fs::TempDir::new().unwrap();
    let state_file = assert_fs::NamedTempFile::new("memocp.db").unwrap();

    source.child("file1.txt").write_str("text file").unwrap();
    source.child("file2.log").write_str("log file").unwrap();
    source
        .child("subdir/file3.txt")
        .write_str("nested text file")
        .unwrap();

    get_cmd()
        .arg(source.path())
        .arg(destination.path())
        .arg("--state-file")
        .arg(state_file.path())
        .arg("--glob")
        .arg("*.txt")
        .assert()
        .success();

    destination
        .child("file1.txt")
        .assert(predicate::path::exists());
    destination
        .child("file2.log")
        .assert(predicate::path::missing());
    destination
        .child("subdir/file3.txt")
        .assert(predicate::path::exists());
}

#[test]
fn test_copy_once() {
    let source = assert_fs::TempDir::new().unwrap();
    let destination = assert_fs::TempDir::new().unwrap();
    let state_file = assert_fs::NamedTempFile::new("memocp.db").unwrap();

    source
        .child("file1.txt")
        .write_str("Hello, world!")
        .unwrap();

    get_cmd()
        .arg(source.path())
        .arg(destination.path())
        .arg("--state-file")
        .arg(state_file.path())
        .assert()
        .success();
    fs::remove_file(destination.child("file1.txt").path()).unwrap();

    get_cmd()
        .arg(source.path())
        .arg(destination.path())
        .arg("--state-file")
        .arg(state_file.path())
        .assert()
        .success();

    destination
        .child("file1.txt")
        .assert(predicate::path::missing());
}

#[test]
fn test_load() {
    let source = assert_fs::TempDir::new().unwrap();
    let destination = assert_fs::TempDir::new().unwrap();
    let state_file = assert_fs::NamedTempFile::new("memocp.db").unwrap();

    source
        .child("file1.txt")
        .write_str("Hello, world!")
        .unwrap();

    get_cmd()
        .arg(source.path())
        .arg(destination.path())
        .arg("--state-file")
        .arg(state_file.path())
        .arg("--load")
        .assert()
        .success();

    destination
        .child("file1.txt")
        .assert(predicate::path::missing());

    get_cmd()
        .arg(source.path())
        .arg(destination.path())
        .arg("--state-file")
        .arg(state_file.path())
        .assert()
        .success();

    destination
        .child("file1.txt")
        .assert(predicate::path::missing());
}

#[test]
fn test_hard_link() {
    let source = assert_fs::TempDir::new().unwrap();
    let destination = assert_fs::TempDir::new().unwrap();
    let state_file = assert_fs::NamedTempFile::new("memocp.db").unwrap();

    let file1 = source.child("file1.txt");
    file1.write_str("Hello, hardlink!").unwrap();

    get_cmd()
        .arg(source.path())
        .arg(destination.path())
        .arg("--state-file")
        .arg(state_file.path())
        .arg("--mode")
        .arg("hard-link")
        .assert()
        .success();

    let dest_file = destination.child("file1.txt");
    dest_file.assert(predicate::path::exists());
    dest_file.assert(predicate::str::diff("Hello, hardlink!"));

    // Hack to indirectly test that the file is hard-linked...
    file1.write_str("Modified source").unwrap();
    dest_file.assert(predicate::str::diff("Modified source"));
}

#[test]
fn test_default_state_file() {
    let current_dir = assert_fs::TempDir::new().unwrap();
    let source = assert_fs::TempDir::new().unwrap();
    let destination = assert_fs::TempDir::new().unwrap();

    get_cmd()
        .current_dir(current_dir.path())
        .arg(source.path())
        .arg(destination.path())
        .assert()
        .success();

    current_dir
        .child("memocp.db")
        .assert(predicate::path::exists());
}

#[test]
fn test_no_override() {
    let source = assert_fs::TempDir::new().unwrap();
    let destination = assert_fs::TempDir::new().unwrap();
    let state_file = assert_fs::NamedTempFile::new("memocp.db").unwrap();

    let file1_source = source.child("file1.txt");
    file1_source.write_str("Source content").unwrap();

    let file1_dest = destination.child("file1.txt");
    file1_dest.write_str("Original content").unwrap();

    get_cmd()
        .arg(source.path())
        .arg(destination.path())
        .arg("--state-file")
        .arg(state_file.path())
        .arg("--mode")
        .arg("copy")
        .assert()
        .success();

    // Verify the destination content is unchanged.
    file1_dest.assert("Original content");
}

#[test]
fn test_override() {
    let source = assert_fs::TempDir::new().unwrap();
    let destination = assert_fs::TempDir::new().unwrap();
    let state_file = assert_fs::NamedTempFile::new("memocp.db").unwrap();

    let file1_source = source.child("file1.txt");
    file1_source.write_str("Source content").unwrap();

    let file1_dest = destination.child("file1.txt");
    file1_dest.write_str("Original content").unwrap();

    get_cmd()
        .arg(source.path())
        .arg(destination.path())
        .arg("--state-file")
        .arg(state_file.path())
        .arg("--mode")
        .arg("copy")
        .arg("--override")
        .assert()
        .success();

    // Verify the destination content is updated.
    file1_dest.assert("Source content");
}

fn get_cmd() -> Command {
    let mut cmd = Command::cargo_bin(BIN_NAME).unwrap();
    cmd.timeout(Duration::from_secs(30));
    cmd
}
