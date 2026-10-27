//! Card 4: Ctrl-N creates a bookmark, choosing its space from the browser's.

mod support;

use support::{Key, World};

const MARKS: &str = r#"# my bookmarks

[docs]
space = "Work"
url = "https://docs.example.com"
"#;

fn world_with_spaces() -> World {
    let world = World::new();
    world.write_config(&format!(
        r#"bookmarks = {:?}

[browser]
type = "command"
command = ["true"]
spaces = ["Work", "Home"]
"#,
        world.bookmarks_path()
    ));
    world.write_bookmarks(MARKS);
    world
}

fn file(world: &World) -> String {
    std::fs::read_to_string(world.bookmarks_path()).unwrap()
}

#[test]
fn ctrl_n_creates_a_bookmark_in_the_chosen_space() {
    let world = world_with_spaces();
    let mut session = world.launch();
    session.expect("docs");
    session.press(Key::Ctrl('n'));
    session.expect("new bookmark");
    session.type_text("calendar");
    session.press(Key::Tab);
    session.type_text("calendar.example.com");
    session.press(Key::Tab);
    session.type_text("dev");
    session.press(Key::Tab);
    session.expect("Home");
    session.press(Key::Down);
    session.press(Key::Enter);
    session.expect_gone("new bookmark");
    session.expect("calendar.example.com");

    let saved = file(&world);
    assert!(
        saved.starts_with(MARKS),
        "existing contents changed:\n{saved}"
    );
    assert!(saved.contains("[calendar]"), "{saved}");
    assert!(saved.contains(r#"space = "Home""#), "{saved}");
    assert!(saved.contains(r#"url = "calendar.example.com""#), "{saved}");
    assert!(saved.contains(r#"tags = ["dev"]"#), "{saved}");
}

#[test]
fn a_duplicate_name_is_refused_and_the_form_stays_open() {
    let world = world_with_spaces();
    let mut session = world.launch();
    session.expect("docs");
    session.press(Key::Ctrl('n'));
    session.expect("new bookmark");
    session.type_text("docs");
    session.press(Key::Tab);
    session.type_text("other.example.com");
    session.press(Key::Enter);
    session.expect("already a bookmark called docs");
    session.expect("new bookmark");
    assert_eq!(file(&world), MARKS);
}

#[test]
fn a_missing_url_is_refused_and_the_form_stays_open() {
    let world = world_with_spaces();
    let mut session = world.launch();
    session.expect("docs");
    session.press(Key::Ctrl('n'));
    session.expect("new bookmark");
    session.type_text("calendar");
    session.press(Key::Enter);
    session.expect("needs a url");
    session.expect("new bookmark");
    assert_eq!(file(&world), MARKS);
}

#[test]
fn esc_cancels_without_writing() {
    let world = world_with_spaces();
    let mut session = world.launch();
    session.expect("docs");
    session.press(Key::Ctrl('n'));
    session.expect("new bookmark");
    session.type_text("calendar");
    session.press(Key::Esc);
    session.expect_gone("new bookmark");
    session.expect("docs");
    assert_eq!(file(&world), MARKS);
}
