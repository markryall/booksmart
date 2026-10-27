//! Flags that answer without starting the TUI, so scripts (and a Homebrew
//! formula's test) can run booksmart without a terminal.

use std::process::Command;

fn booksmart(arg: &str) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_booksmart"))
        .arg(arg)
        .output()
        .unwrap()
}

#[test]
fn version_prints_the_version() {
    let out = booksmart("--version");
    assert!(out.status.success());
    assert_eq!(
        String::from_utf8(out.stdout).unwrap(),
        format!("booksmart {}\n", env!("CARGO_PKG_VERSION"))
    );
}

#[test]
fn help_describes_usage() {
    let out = booksmart("--help");
    assert!(out.status.success());
    assert!(
        String::from_utf8(out.stdout)
            .unwrap()
            .contains("BOOKSMART_CONFIG")
    );
}

#[test]
fn an_unknown_argument_is_an_error() {
    let out = booksmart("--nope");
    assert!(!out.status.success());
    assert!(String::from_utf8(out.stderr).unwrap().contains("--nope"));
}
