use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_cli_help() {
    let mut cmd = Command::cargo_bin("ms").unwrap();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage:"));
}

#[test]
fn test_cli_version() {
    let mut cmd = Command::cargo_bin("ms").unwrap();
    cmd.arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn test_robot_mode_global() {
    let mut cmd = Command::cargo_bin("ms").unwrap();
    cmd.args(["--robot", "--help"]).assert().success();
}
