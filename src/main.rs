use std::time::Duration;

use anyhow::Result;
use booksmart::app::App;
use booksmart::config::Config;
use ratatui::DefaultTerminal;
use ratatui::crossterm::event::{self, Event, KeyEventKind};

fn main() -> Result<()> {
    let config = Config::load()?;
    let mut terminal = ratatui::init();
    let result = run(&mut terminal, App::new(config));
    ratatui::restore();
    result
}

fn run(terminal: &mut DefaultTerminal, mut app: App) -> Result<()> {
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
    Ok(())
}
