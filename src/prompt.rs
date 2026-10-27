//! Asks for the values a templated bookmark needs before it opens.

use ratatui::Frame;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::style::{Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Clear, Paragraph, Wrap};

use crate::bookmarks::Bookmark;
use crate::popup::centered;
use crate::template;

pub enum Action {
    Continue,
    Cancel,
    /// The bookmark with its urls filled in.
    Open(Bookmark),
}

pub struct Prompt {
    bookmark: Bookmark,
    input: String,
    error: Option<String>,
}

impl Prompt {
    pub fn new(bookmark: Bookmark) -> Prompt {
        Prompt {
            bookmark,
            input: String::new(),
            error: None,
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Action {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        match key.code {
            KeyCode::Esc => return Action::Cancel,
            KeyCode::Enter => match template::fill_all(&self.bookmark.urls, &self.input) {
                Ok(urls) => {
                    return Action::Open(Bookmark {
                        urls,
                        ..self.bookmark.clone()
                    });
                }
                Err(message) => self.error = Some(message),
            },
            KeyCode::Char('u') if ctrl => self.input.clear(),
            KeyCode::Backspace => {
                self.input.pop();
            }
            KeyCode::Char(c) if !ctrl => {
                self.input.push(c);
                self.error = None;
            }
            _ => {}
        }
        Action::Continue
    }

    pub fn render(&self, frame: &mut Frame) {
        let area = centered(frame.area(), 80, 6 + self.bookmark.urls.len() as u16);
        frame.render_widget(Clear, area);

        let mut lines = vec![Line::from(vec![
            Span::from("values  ").bold(),
            Span::from(self.input.clone()),
            Span::from("▏").dim(),
        ])];
        // each url as it would open, or as written while it can't be filled yet
        for url in &self.bookmark.urls {
            let preview = template::fill(url, &self.input).unwrap_or_else(|_| url.clone());
            lines.push(Line::from(vec![
                Span::from("   url  ").dim(),
                Span::from(preview).cyan(),
            ]));
        }
        lines.push(Line::default());
        match &self.error {
            Some(error) => lines.push(Line::from(error.clone()).red()),
            None => lines.push(
                Line::from("words fill {1} {2}.. · {*} takes the rest · enter open · esc back")
                    .dim(),
            ),
        }

        let block = Block::bordered()
            .title(format!("open {}", self.bookmark.name))
            .border_style(Style::new().cyan());
        frame.render_widget(
            Paragraph::new(lines)
                .wrap(Wrap { trim: false })
                .block(block),
            area,
        );
    }
}
