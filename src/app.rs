//! The TUI's state and how it draws, kept apart from the terminal loop so it
//! can be driven directly in tests.

use ratatui::Frame;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Style, Stylize};
use ratatui::text::Line;
use ratatui::widgets::{Block, Paragraph};

use crate::config::Config;

pub struct App {
    pub config: Config,
    pub should_quit: bool,
}

impl App {
    pub fn new(config: Config) -> App {
        App {
            config,
            should_quit: false,
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.should_quit = true
            }
            _ => {}
        }
    }

    pub fn render(&self, frame: &mut Frame) {
        let [header, body, footer] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Fill(1),
            Constraint::Length(1),
        ])
        .areas(frame.area());

        frame.render_widget(Line::from("booksmart".bold()), header);

        let hint = Paragraph::new(vec![
            Line::from("no bookmarks yet"),
            Line::from(format!("add some to {}", self.config.bookmarks.display()).dim()),
        ])
        .block(Block::bordered());
        frame.render_widget(hint, body);

        frame.render_widget(Line::from("q quit").style(Style::new().dim()), footer);
    }
}
