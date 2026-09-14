//! Integration tests that exercise the compiled binary end to end.

use std::fs;
use std::io::Write;
use std::process::{Command, Output, Stdio};

fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_hidenv")
}

/// Writes a temp `.env` file and returns its path.
fn temp_env(content: &str) -> String {
    let dir = std::env::temp_dir().join(format!("hidenv-test-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join("sample.env");
    let mut fh = fs::File::create(&path).unwrap();
    fh.write_all(content.as_bytes()).unwrap();
    path.to_string_lossy().into_owned()
}

fn run(args: &[&str]) -> Output {
    Command::new(binary())
        .args(args)
        .output()
        .expect("failed to run hidenv")
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

const SAMPLE: &str = "DB_HOST=localhost\nDB_PASSWORD=supersecret\nAPI_KEY=abc123\n# a comment\n";

#[test]
fn masks_values_by_default() {
    let file = temp_env(SAMPLE);
    let out = run(&[&file]);
    assert!(out.status.success());
    assert_eq!(
        stdout(&out),
        "DB_HOST=###\nDB_PASSWORD=###\nAPI_KEY=###\n# a comment\n"
    );
}

#[test]
fn masks_stdin() {
    let mut child = Command::new(binary())
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(b"TOKEN=shh\n")
        .unwrap();
    let out = child.wait_with_output().unwrap();
    assert_eq!(stdout(&out), "TOKEN=###\n");
}

#[test]
fn lists_only_keys() {
    let file = temp_env(SAMPLE);
    let out = run(&[&file, "--keys"]);
    assert!(out.status.success());
    assert_eq!(stdout(&out), "DB_HOST\nDB_PASSWORD\nAPI_KEY\n");
}

#[test]
fn shows_requested_keys() {
    let file = temp_env(SAMPLE);
    let out = run(&[&file, "--show", "API_KEY"]);
    assert!(out.status.success());
    assert_eq!(stdout(&out), "API_KEY=abc123\n");
}

#[test]
fn shows_all_keys() {
    let file = temp_env(SAMPLE);
    let out = run(&[&file, "--all"]);
    assert!(out.status.success());
    assert_eq!(
        stdout(&out),
        "DB_HOST=localhost\nDB_PASSWORD=supersecret\nAPI_KEY=abc123\n"
    );
}

#[test]
fn reports_missing_key() {
    let file = temp_env(SAMPLE);
    let out = run(&[&file, "--show", "NOPE"]);
    assert!(!out.status.success());
    assert!(stderr(&out).contains("key not found: NOPE"));
}

#[test]
fn masks_inline_variable_args() {
    let out = run(&["MINHA_VAR=123"]);
    assert!(out.status.success());
    assert_eq!(stdout(&out), "MINHA_VAR=###\n");
}

#[test]
fn masks_multiple_inline_variables() {
    let out = run(&["A=1", "B = 2"]);
    assert!(out.status.success());
    assert_eq!(stdout(&out), "A=###\nB=###\n");
}

#[test]
fn no_args_masks_shell_environment() {
    let out = run(&[]);
    assert!(out.status.success());
    let text = stdout(&out);
    assert!(text.lines().any(|l| l == "PATH=###"));
    assert!(text.lines().all(|l| l.ends_with("=###")));
}

#[test]
fn keys_without_file_lists_environment() {
    let out = run(&["--keys"]);
    assert!(out.status.success());
    assert!(stdout(&out).lines().any(|l| l == "PATH"));
}

#[test]
fn help_flag_succeeds() {
    let out = run(&["--help"]);
    assert!(out.status.success());
    assert!(stdout(&out).contains("hidenv"));
}

#[test]
fn unknown_flag_fails() {
    let out = run(&["--nope"]);
    assert!(!out.status.success());
    assert!(stderr(&out).contains("unknown flag"));
}

#[test]
fn invalid_ssh_target_fails() {
    let out = run(&["-S", ":missing-host"]);
    assert!(!out.status.success());
    assert!(stderr(&out).contains("invalid SSH target"));
}
