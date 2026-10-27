//! Card 2: bookmarks in the file show up, and keep up with changes on disk.

mod support;

use support::{Key, World};

const TWO: &str = r#"
# a comment, to show the file is ours to annotate
[mail-work]
space = "Work"
url = "https://mail.example.com"

[docs]
space = "Home"
url = ["https://docs.example.com", "https://wiki.example.com"]
tags = ["dev"]
"#;

#[test]
fn bookmarks_in_the_file_are_listed_with_name_space_and_url() {
    let world = World::new();
    world.write_bookmarks(TWO);
    let session = world.launch();
    session.expect("mail-work");
    session.expect("Work");
    session.expect("https://mail.example.com");
    session.expect("docs");
    session.expect("https://docs.example.com");
    session.expect_gone("no bookmarks yet");
}

#[test]
fn a_bookmark_added_while_running_appears_without_a_keypress() {
    let world = World::new();
    world.write_bookmarks(TWO);
    let session = world.launch();
    session.expect("mail-work");
    world.write_bookmarks(&format!(
        "{TWO}\n[calendar]\nspace = \"Work\"\nurl = \"https://calendar.example.com\"\n"
    ));
    session.expect("calendar");
}

#[test]
fn an_invalid_file_shows_an_error_but_keeps_the_last_good_list() {
    let world = World::new();
    world.write_bookmarks(TWO);
    let session = world.launch();
    session.expect("mail-work");
    world.write_bookmarks("[broken\nspace = ");
    session.expect("can't read bookmarks");
    session.expect("mail-work");
    world.write_bookmarks(TWO);
    session.expect_gone("can't read bookmarks");
}

#[test]
fn typing_filters_fuzzily_and_backspace_widens_again() {
    let world = World::new();
    world.write_bookmarks(TWO);
    let mut session = world.launch();
    session.expect("mail-work");
    session.type_text("mailw");
    session.expect_gone("docs.example.com");
    session.expect("mail-work");
    for _ in "mailw".chars() {
        session.press(Key::Backspace);
    }
    session.expect("docs.example.com");
}
