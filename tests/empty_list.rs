//! Card 1: open the ui and see an empty list.

mod support;

use support::World;

#[test]
fn empty_bookmarks_file_shows_header_and_hint() {
    let world = World::new();
    world.write_bookmarks("");
    let session = world.launch();
    session.expect("booksmart");
    session.expect("no bookmarks yet");
}

#[test]
fn missing_bookmarks_file_is_treated_as_empty() {
    let world = World::new();
    let session = world.launch();
    session.expect("no bookmarks yet");
}

#[test]
fn q_quits_cleanly_and_restores_the_terminal() {
    let world = World::new();
    let mut session = world.launch();
    session.expect("no bookmarks yet");
    session.type_text("q");
    assert!(
        session.wait_exit(),
        "booksmart exited with a failure status"
    );
    assert!(
        session.restored(),
        "terminal was left on the alternate screen"
    );
}
