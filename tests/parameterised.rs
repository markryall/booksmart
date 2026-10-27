//! Card 6: bookmarks with {1} {2}.. and {*} placeholders prompt for values.

mod support;

use support::{Key, World, eventually};

const MARKS: &str = r#"
[supplier]
space = "Work"
url = "https://shop.example.com/suppliers/{1}/orders/{2}"

[search]
space = "Home"
url = "https://search.example.com/?q={*}&order=name"
"#;

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
fn positional_values_are_prompted_for_and_filled() {
    let world = recording_world();
    let mut session = world.launch();
    session.expect("supplier");
    session.press(Key::Enter);
    session.expect("open supplier");
    session.type_text("42 7");
    session.expect("https://shop.example.com/suppliers/42/orders/7");
    session.press(Key::Enter);
    eventually("the bookmark opened", || opened(&world).len() == 1);
    assert_eq!(
        opened(&world),
        ["Work|https://shop.example.com/suppliers/42/orders/7"]
    );
}

#[test]
fn star_takes_the_whole_input_escaped() {
    let world = recording_world();
    let mut session = world.launch();
    session.expect("search");
    session.press(Key::Down);
    session.press(Key::Enter);
    session.expect("open search");
    session.type_text("sci who & co");
    session.press(Key::Enter);
    eventually("the bookmark opened", || opened(&world).len() == 1);
    assert_eq!(
        opened(&world),
        ["Home|https://search.example.com/?q=sci%20who%20%26%20co&order=name"]
    );
}

#[test]
fn a_missing_value_is_named_and_nothing_opens() {
    let world = recording_world();
    let mut session = world.launch();
    session.expect("supplier");
    session.press(Key::Enter);
    session.expect("open supplier");
    session.type_text("42");
    session.press(Key::Enter);
    session.expect("needs a value for {2}");
    session.expect("open supplier");
    assert!(opened(&world).is_empty());
}

#[test]
fn esc_goes_back_to_the_list() {
    let world = recording_world();
    let mut session = world.launch();
    session.expect("supplier");
    session.press(Key::Enter);
    session.expect("open supplier");
    session.press(Key::Esc);
    session.expect_gone("open supplier");
    session.expect("search.example.com");
    assert!(opened(&world).is_empty());
}
