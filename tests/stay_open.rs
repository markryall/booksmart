//! Card 8: Enter opens a bookmark and booksmart keeps running.

mod support;

use support::{Key, World, eventually};

const MARKS: &str = r#"
[mail-work]
space = "Work"
url = "https://mail.example.com"

[docs]
space = "Home"
url = "https://docs.example.com"
"#;

/// Points the world's browser at `command`.
fn with_command(world: World, command: &str) -> World {
    world.write_config(&format!(
        "bookmarks = {:?}\n\n[browser]\ntype = \"command\"\ncommand = {command}\n",
        world.bookmarks_path(),
    ));
    world.write_bookmarks(MARKS);
    world
}

fn recording_world() -> World {
    let world = World::new();
    let log = world.path("opened.log");
    with_command(
        world,
        &format!(
            r#"["sh", "-c", "echo noise; printf '%s|%s\n' \"$1\" \"$2\" >> {}", "sh", "{{space}}", "{{url}}"]"#,
            log.display()
        ),
    )
}

fn opened(world: &World) -> Vec<String> {
    std::fs::read_to_string(world.path("opened.log"))
        .unwrap_or_default()
        .lines()
        .map(str::to_string)
        .collect()
}

#[test]
fn enter_opens_and_the_list_stays_on_screen() {
    let world = recording_world();
    let mut session = world.launch();
    session.expect("mail-work");
    session.press(Key::Enter);
    eventually("the bookmark opened", || opened(&world).len() == 1);
    session.expect("opened mail-work in Work");
    session.expect("docs.example.com");
    assert!(session.running(), "booksmart quit after opening");
    assert!(
        !session.contents().contains("noise"),
        "the browser's output leaked onto the screen"
    );
}

#[test]
fn two_bookmarks_opened_one_after_the_other_both_open() {
    let world = recording_world();
    let mut session = world.launch();
    session.expect("mail-work");
    session.press(Key::Enter);
    eventually("the first bookmark opened", || opened(&world).len() == 1);
    session.press(Key::Down);
    session.press(Key::Enter);
    eventually("the second bookmark opened", || opened(&world).len() == 2);
    assert_eq!(
        opened(&world),
        [
            "Work|https://mail.example.com",
            "Home|https://docs.example.com"
        ]
    );
}

#[test]
fn the_filter_clears_after_opening() {
    let world = recording_world();
    let mut session = world.launch();
    session.expect("mail-work");
    session.type_text("docs");
    session.expect_gone("mail.example.com");
    session.press(Key::Enter);
    eventually("the bookmark opened", || opened(&world).len() == 1);
    session.expect("mail.example.com");
}

#[test]
fn a_failed_open_shows_the_error_and_keeps_running() {
    let world = with_command(World::new(), r#"["sh", "-c", "echo boom >&2; exit 3"]"#);
    let mut session = world.launch();
    session.expect("mail-work");
    session.press(Key::Enter);
    session.expect("boom");
    assert!(session.running(), "booksmart quit after a failed open");
}

#[test]
fn esc_still_quits() {
    let world = recording_world();
    let mut session = world.launch();
    session.expect("mail-work");
    session.press(Key::Enter);
    eventually("the bookmark opened", || opened(&world).len() == 1);
    session.press(Key::Esc);
    assert!(
        session.wait_exit(),
        "booksmart exited with a failure status"
    );
}
