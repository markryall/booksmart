use std::time::Duration;

use anyhow::{Result, bail};
use booksmart::app::App;
use booksmart::bookmarks::Bookmark;
use booksmart::config::Config;
use ratatui::DefaultTerminal;
use ratatui::crossterm::event::{self, Event, KeyEventKind};

const HELP: &str = "\
booksmart: fuzzy-filter your bookmarks and open them in the right browser space

usage: booksmart [--help | --version]

Reads ~/.config/booksmart/config.toml, or the file $BOOKSMART_CONFIG names.
See https://github.com/markryall/booksmart for the bookmark and config format.
";

fn main() -> Result<()> {
    match std::env::args().nth(1).as_deref() {
        None => {}
        Some("-h" | "--help") => {
            print!("{HELP}");
            return Ok(());
        }
        Some("-V" | "--version") => {
            println!("booksmart {}", env!("CARGO_PKG_VERSION"));
            return Ok(());
        }
        Some(other) => bail!("unknown argument {other} (try --help)"),
    }

    let config = Config::load()?;
    let mut terminal = ratatui::init();
    let chosen = run(&mut terminal, App::new(config.clone()));
    ratatui::restore();
    if let Some(bookmark) = chosen? {
        config.browser.open(&bookmark.space, &bookmark.urls)?;
    }
    Ok(())
}

/// Runs the TUI until it quits, answering the bookmark picked to open, if any.
fn run(terminal: &mut DefaultTerminal, mut app: App) -> Result<Option<Bookmark>> {
    while !app.should_quit {
        app.reload();
        terminal.draw(|frame| app.render(frame))?;
        // wake up regularly even without input, so changes on disk show by themselves
        if event::poll(Duration::from_millis(250))?
            && let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
        {
            app.handle_key(key);
        }
    }
    Ok(app.chosen)
}
