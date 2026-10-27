//! Card 3: Enter opens the selected bookmark in its space.

mod support;

use support::{Key, World};

const MARKS: &str = r#"
[mail-work]
space = "Work"
url = "https://mail.example.com"

[docs]
space = "Home"
url = ["https://docs.example.com", "wiki.example.com"]
"#;

/// A world whose browser is a command that logs each open as `space|url`.
fn recording_world() -> World {
    let world = World::new();
    let log = world.path("opened.log");
    world.write_config(&format!(
        r#"bookmarks = {bookmarks:?}

[browser]
type = "command"
command = ["sh", "-c", "printf '%s|%s\n' \"$1\" \"$2\" >> {log}", "sh", "{{space}}", "{{url}}"]
"#,
        bookmarks = world.bookmarks_path(),
        log = log.display(),
    ));
    world.write_bookmarks(MARKS);
    world
}

fn opened(world: &World) -> Vec<String> {
    std::fs::read_to_string(world.path("opened.log"))
        .unwrap_or_default()
        .lines()
        .map(str::to_string)
        .collect()
}

#[test]
fn enter_opens_the_selected_bookmark_in_its_space_and_quits() {
    let world = recording_world();
    let mut session = world.launch();
    session.expect("mail-work");
    session.press(Key::Enter);
    assert!(
        session.wait_exit(),
        "booksmart exited with a failure status"
    );
    assert_eq!(opened(&world), ["Work|https://mail.example.com"]);
}

#[test]
fn a_multi_url_bookmark_opens_every_url_in_order() {
    let world = recording_world();
    let mut session = world.launch();
    session.expect("docs");
    session.press(Key::Down);
    session.press(Key::Enter);
    assert!(
        session.wait_exit(),
        "booksmart exited with a failure status"
    );
    assert_eq!(
        opened(&world),
        [
            "Home|https://docs.example.com",
            "Home|https://wiki.example.com"
        ]
    );
}

#[test]
fn enter_opens_the_top_match_of_a_filter() {
    let world = recording_world();
    let mut session = world.launch();
    session.expect("docs");
    session.type_text("docs");
    session.expect_gone("mail.example.com");
    session.press(Key::Enter);
    assert!(
        session.wait_exit(),
        "booksmart exited with a failure status"
    );
    assert_eq!(opened(&world)[0], "Home|https://docs.example.com");
}
