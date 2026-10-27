use std::time::Duration;

use anyhow::Result;
use booksmart::app::App;
use booksmart::bookmarks::{self, Bookmark};
use booksmart::cli::{self, Invocation};
use booksmart::config::Config;
use ratatui::DefaultTerminal;
use ratatui::crossterm::event::{self, Event, KeyEventKind};

const HELP: &str = "\
booksmart: fuzzy-filter your bookmarks and open them in the right browser space

usage: booksmart                    pick a bookmark in the TUI
       booksmart <name> [values..]  open one straight away, filling {1} {2}.. {*}
       booksmart --help | --version

Reads ~/.config/booksmart/config.toml, or the file $BOOKSMART_CONFIG names.
See https://github.com/markryall/booksmart for the bookmark and config format.
";

fn main() -> Result<()> {
    match cli::parse(std::env::args().skip(1))? {
        Invocation::Help => print!("{HELP}"),
        Invocation::Version => println!("booksmart {}", env!("CARGO_PKG_VERSION")),
        Invocation::Open { name, values } => {
            let config = Config::load()?;
            let marks = bookmarks::parse(&bookmarks::read(&config.bookmarks)?)?;
            let bookmark = cli::resolve(&marks, &name, &values)?;
            config.browser.open(&bookmark.space, &bookmark.urls)?;
        }
        Invocation::Tui => {
            let config = Config::load()?;
            let mut terminal = ratatui::init();
            let chosen = run(&mut terminal, App::new(config.clone()));
            ratatui::restore();
            if let Some(bookmark) = chosen? {
                config.browser.open(&bookmark.space, &bookmark.urls)?;
            }
        }
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
