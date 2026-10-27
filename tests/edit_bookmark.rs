//! Card 5: Ctrl-E edits the selected bookmark in place.

mod support;

use support::{Key, World};

const MARKS: &str = r#"# my bookmarks

[docs]   # the team docs
space = "Work"
url = "https://docs.example.com"

# personal
[mail]
space = "Home"
url = "https://mail.example.com"
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
fn ctrl_e_changes_the_url_and_space() {
    let world = world_with_spaces();
    let mut session = world.launch();
    session.expect("docs");
    session.press(Key::Ctrl('e'));
    session.expect("edit bookmark");
    session.expect("https://docs.example.com");
    session.press(Key::Tab);
    session.press(Key::Ctrl('u'));
    session.type_text("https://wiki.example.com");
    session.press(Key::Tab);
    session.press(Key::Tab);
    session.press(Key::Down);
    session.press(Key::Enter);
    session.expect_gone("edit bookmark");
    session.expect("https://wiki.example.com");
    session.expect_gone("https://docs.example.com");

    let saved = file(&world);
    let docs = saved.split("# personal").next().unwrap();
    assert!(docs.contains(r#"space = "Home""#), "{saved}");
    assert!(
        docs.contains(r#"url = "https://wiki.example.com""#),
        "{saved}"
    );
    assert!(saved.contains("[docs]   # the team docs"), "{saved}");
}

#[test]
fn renaming_moves_the_table_and_keeps_its_place() {
    let world = world_with_spaces();
    let mut session = world.launch();
    session.expect("docs");
    session.press(Key::Ctrl('e'));
    session.expect("edit bookmark");
    session.press(Key::Ctrl('u'));
    session.type_text("notes");
    session.press(Key::Enter);
    session.expect_gone("edit bookmark");
    session.expect("notes");

    let saved = file(&world);
    assert!(!saved.contains("[docs]"), "{saved}");
    let notes = saved.find("[notes]").expect("renamed table");
    let mail = saved.find("[mail]").unwrap();
    assert!(notes < mail, "renamed table moved:\n{saved}");
}

#[test]
fn renaming_to_an_existing_name_is_refused() {
    let world = world_with_spaces();
    let mut session = world.launch();
    session.expect("docs");
    session.press(Key::Ctrl('e'));
    session.expect("edit bookmark");
    session.press(Key::Ctrl('u'));
    session.type_text("mail");
    session.press(Key::Enter);
    session.expect("already a bookmark called mail");
    session.expect("edit bookmark");
    assert_eq!(file(&world), MARKS);
}

#[test]
fn saving_unchanged_leaves_the_file_alone() {
    let world = world_with_spaces();
    let mut session = world.launch();
    session.expect("docs");
    session.press(Key::Ctrl('e'));
    session.expect("edit bookmark");
    session.press(Key::Enter);
    session.expect_gone("edit bookmark");
    assert_eq!(file(&world), MARKS);
}
