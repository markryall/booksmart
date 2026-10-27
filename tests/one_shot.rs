//! Card 7: `booksmart <name> [values..]` opens a bookmark without the TUI.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use tempfile::TempDir;

const MARKS: &str = r#"
[mail-work]
space = "Work"
url = "https://mail.example.com"

[mail-home]
space = "Home"
url = "https://mail.example.com/u/1"

[supplier]
space = "Work"
url = "https://shop.example.com/suppliers/{1}/orders/{2}"

[search]
space = "Home"
url = "https://search.example.com/{1}?q={*}"
"#;

struct World {
    dir: TempDir,
}

impl World {
    fn new() -> World {
        let world = World {
            dir: TempDir::new().unwrap(),
        };
        std::fs::write(world.path("bookmarks.toml"), MARKS).unwrap();
        std::fs::write(
            world.path("config.toml"),
            format!(
                r#"bookmarks = {bookmarks:?}

[browser]
type = "command"
command = ["sh", "-c", "printf '%s|%s\n' \"$1\" \"$2\" >> {log}", "sh", "{{space}}", "{{url}}"]
"#,
                bookmarks = world.path("bookmarks.toml"),
                log = world.path("opened.log").display(),
            ),
        )
        .unwrap();
        world
    }

    fn path(&self, name: &str) -> PathBuf {
        self.dir.path().join(name)
    }

    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_booksmart"))
            .args(args)
            .env("BOOKSMART_CONFIG", self.path("config.toml"))
            .env("HOME", self.dir.path())
            .output()
            .unwrap()
    }

    fn opened(&self) -> Vec<String> {
        read_lines(&self.path("opened.log"))
    }
}

fn read_lines(path: &Path) -> Vec<String> {
    std::fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .map(str::to_string)
        .collect()
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

#[test]
fn a_name_opens_that_bookmark_without_the_tui() {
    let world = World::new();
    let out = world.run(&["mail-work"]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(world.opened(), ["Work|https://mail.example.com"]);
    // the alternate screen is how the TUI starts; a one-shot never enters it
    assert!(!String::from_utf8_lossy(&out.stdout).contains("\x1b[?1049h"));
}

#[test]
fn values_fill_positional_placeholders_in_order() {
    let world = World::new();
    let out = world.run(&["supplier", "42", "7"]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(
        world.opened(),
        ["Work|https://shop.example.com/suppliers/42/orders/7"]
    );
}

#[test]
fn extra_values_go_to_star_escaped() {
    let world = World::new();
    let out = world.run(&["search", "people", "sci", "who & co"]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(
        world.opened(),
        ["Home|https://search.example.com/people?q=sci%20who%20%26%20co"]
    );
}

#[test]
fn an_unknown_name_fails_and_suggests_close_ones() {
    let world = World::new();
    let out = world.run(&["mail"]);
    assert!(!out.status.success());
    let err = stderr(&out);
    assert!(err.contains("no bookmark called mail"), "{err}");
    assert!(err.contains("mail-work"), "{err}");
    assert!(err.contains("mail-home"), "{err}");
    assert!(world.opened().is_empty());
}

#[test]
fn a_missing_value_fails_naming_the_placeholder_and_opens_nothing() {
    let world = World::new();
    let out = world.run(&["supplier", "42"]);
    assert!(!out.status.success());
    assert!(
        stderr(&out).contains("needs a value for {2}"),
        "{}",
        stderr(&out)
    );
    assert!(world.opened().is_empty());
}

#[test]
fn values_for_a_bookmark_without_placeholders_are_refused() {
    let world = World::new();
    let out = world.run(&["mail-work", "oops"]);
    assert!(!out.status.success());
    assert!(
        stderr(&out).contains("mail-work takes no values"),
        "{}",
        stderr(&out)
    );
    assert!(world.opened().is_empty());
}
