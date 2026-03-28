use assert_cmd::Command;
use predicates::prelude::predicate;
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

fn get_cmd() -> Command {
    let mut cmd = Command::cargo_bin(BIN_NAME).unwrap();
    cmd.timeout(Duration::from_secs(30));
    cmd
}
