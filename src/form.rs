//! The form for creating (and later editing) a bookmark.

use ratatui::Frame;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::{Constraint, Flex, Layout, Rect};
use ratatui::style::{Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Clear, Paragraph};

use crate::bookmarks::Bookmark;

#[derive(Debug, Clone, Copy, PartialEq)]
enum Field {
    Name,
    Urls,
    Tags,
    Space,
}

const FIELDS: [Field; 4] = [Field::Name, Field::Urls, Field::Tags, Field::Space];

pub enum Action {
    Continue,
    Cancel,
    Submit,
}

pub struct Form {
    title: &'static str,
    name: String,
    urls: String,
    tags: String,
    /// The space typed in, when the browser has no spaces to pick from.
    space_text: String,
    spaces: Vec<String>,
    space_index: usize,
    focus: Field,
    pub error: Option<String>,
}

impl Form {
    /// A blank form. `spaces` is what the space field picks from; when there
    /// are none, the space is typed in instead.
    pub fn new(spaces: Vec<String>, spaces_error: Option<String>) -> Form {
        Form {
            title: "new bookmark",
            name: String::new(),
            urls: String::new(),
            tags: String::new(),
            space_text: String::new(),
            spaces,
            space_index: 0,
            focus: Field::Name,
            error: spaces_error.map(|e| format!("couldn't load spaces: {e}")),
        }
    }

    /// The bookmark as entered: urls and tags are separated by whitespace.
    pub fn bookmark(&self) -> Bookmark {
        let space = match self.spaces.get(self.space_index) {
            Some(space) => space.clone(),
            None => self.space_text.trim().to_string(),
        };
        Bookmark {
            name: self.name.trim().to_string(),
            space,
            urls: self.urls.split_whitespace().map(str::to_string).collect(),
            tags: self.tags.split_whitespace().map(str::to_string).collect(),
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Action {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        match key.code {
            KeyCode::Esc => return Action::Cancel,
            KeyCode::Enter => return Action::Submit,
            KeyCode::Tab => self.move_focus(1),
            KeyCode::BackTab => self.move_focus(FIELDS.len() - 1),
            KeyCode::Up if self.picking() => self.space_index = self.space_index.saturating_sub(1),
            KeyCode::Down if self.picking() => {
                self.space_index = (self.space_index + 1).min(self.spaces.len() - 1);
            }
            KeyCode::Backspace => {
                if let Some(text) = self.text_mut() {
                    text.pop();
                }
            }
            KeyCode::Char(c) if !ctrl => {
                if let Some(text) = self.text_mut() {
                    text.push(c);
                }
            }
            _ => {}
        }
        Action::Continue
    }

    fn picking(&self) -> bool {
        self.focus == Field::Space && !self.spaces.is_empty()
    }

    fn move_focus(&mut self, by: usize) {
        let at = FIELDS.iter().position(|f| *f == self.focus).unwrap_or(0);
        self.focus = FIELDS[(at + by) % FIELDS.len()];
    }

    fn text_mut(&mut self) -> Option<&mut String> {
        match self.focus {
            Field::Name => Some(&mut self.name),
            Field::Urls => Some(&mut self.urls),
            Field::Tags => Some(&mut self.tags),
            Field::Space if self.spaces.is_empty() => Some(&mut self.space_text),
            Field::Space => None,
        }
    }

    pub fn render(&self, frame: &mut Frame) {
        let area = centered(frame.area(), 70, 9);
        frame.render_widget(Clear, area);

        let row = |field: Field, label: &str, value: Vec<Span<'static>>| {
            let focused = self.focus == field;
            let label = Span::from(format!("{label:>6}  "));
            let label = if focused { label.bold() } else { label.dim() };
            let mut spans = vec![label];
            spans.extend(value);
            // a text cursor, except on the space picker
            if focused && (field != Field::Space || self.spaces.is_empty()) {
                spans.push(Span::from("▏").dim());
            }
            Line::from(spans)
        };
        let text = |s: &str| vec![Span::from(s.to_string())];

        let space_value = if self.spaces.is_empty() {
            text(&self.space_text)
        } else {
            self.spaces
                .iter()
                .enumerate()
                .flat_map(|(i, s)| {
                    let span = Span::from(s.clone());
                    let span = if i == self.space_index {
                        span.reversed()
                    } else {
                        span.dim()
                    };
                    [span, Span::from(" ")]
                })
                .collect()
        };

        let mut lines = vec![
            row(Field::Name, "name", text(&self.name)),
            row(Field::Urls, "urls", text(&self.urls)),
            row(Field::Tags, "tags", text(&self.tags)),
            row(Field::Space, "space", space_value),
            Line::default(),
        ];
        match &self.error {
            Some(error) => lines.push(Line::from(error.clone()).red()),
            None => lines
                .push(Line::from("tab next field · ↑↓ pick space · enter save · esc cancel").dim()),
        }

        let block = Block::bordered()
            .title(self.title)
            .border_style(Style::new().cyan());
        frame.render_widget(Paragraph::new(lines).block(block), area);
    }
}

fn centered(area: Rect, width: u16, height: u16) -> Rect {
    let [area] = Layout::horizontal([Constraint::Length(width)])
        .flex(Flex::Center)
        .areas(area);
    let [area] = Layout::vertical([Constraint::Length(height)])
        .flex(Flex::Center)
        .areas(area);
    area
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::crossterm::event::KeyEvent;

    fn press(form: &mut Form, code: KeyCode) -> Action {
        form.handle_key(KeyEvent::from(code))
    }

    fn type_text(form: &mut Form, text: &str) {
        for c in text.chars() {
            press(form, KeyCode::Char(c));
        }
    }

    #[test]
    fn fields_fill_in_order_and_split_on_whitespace() {
        let mut form = Form::new(vec!["Work".into(), "Home".into()], None);
        type_text(&mut form, " calendar ");
        press(&mut form, KeyCode::Tab);
        type_text(&mut form, "a.example.com  b.example.com");
        press(&mut form, KeyCode::Tab);
        type_text(&mut form, "dev ops");
        press(&mut form, KeyCode::Tab);
        press(&mut form, KeyCode::Down);
        assert_eq!(
            form.bookmark(),
            Bookmark {
                name: "calendar".into(),
                space: "Home".into(),
                urls: vec!["a.example.com".into(), "b.example.com".into()],
                tags: vec!["dev".into(), "ops".into()],
            }
        );
    }

    #[test]
    fn the_space_picker_stops_at_either_end() {
        let mut form = Form::new(vec!["Work".into(), "Home".into()], None);
        form.focus = Field::Space;
        press(&mut form, KeyCode::Up);
        assert_eq!(form.bookmark().space, "Work");
        press(&mut form, KeyCode::Down);
        press(&mut form, KeyCode::Down);
        assert_eq!(form.bookmark().space, "Home");
    }

    #[test]
    fn without_spaces_the_space_is_typed() {
        let mut form = Form::new(vec![], None);
        form.focus = Field::Space;
        type_text(&mut form, "Work");
        assert_eq!(form.bookmark().space, "Work");
    }

    #[test]
    fn backtab_wraps_to_the_last_field() {
        let mut form = Form::new(vec![], None);
        press(&mut form, KeyCode::BackTab);
        assert_eq!(form.focus, Field::Space);
    }
}
