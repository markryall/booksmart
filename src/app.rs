//! The TUI's state and how it draws, kept apart from the terminal loop so it
//! can be driven directly in tests.

use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Matcher, Utf32Str};
use ratatui::Frame;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, List, ListItem, ListState, Paragraph};

use crate::bookmarks::{self, Bookmark};
use crate::config::Config;
use crate::form::{self, Form};

pub struct App {
    pub config: Config,
    pub should_quit: bool,
    /// The bookmark Enter picked, to open once the terminal is restored.
    pub chosen: Option<Bookmark>,
    bookmarks: Vec<Bookmark>,
    /// The file's text as last read, so a reload only reparses when it changed.
    last_read: Option<String>,
    error: Option<String>,
    query: String,
    selected: usize,
    matcher: Matcher,
    form: Option<Form>,
}

impl App {
    pub fn new(config: Config) -> App {
        let mut app = App {
            config,
            should_quit: false,
            chosen: None,
            bookmarks: Vec::new(),
            last_read: None,
            error: None,
            query: String::new(),
            selected: 0,
            matcher: Matcher::default(),
            form: None,
        };
        app.reload();
        app
    }

    /// Rereads the bookmarks file. When it no longer parses, the last good
    /// list stays and the error is shown instead.
    pub fn reload(&mut self) {
        let text = match bookmarks::read(&self.config.bookmarks) {
            Ok(text) => text,
            Err(e) => {
                self.error = Some(format!("{e:#}"));
                return;
            }
        };
        if self.last_read.as_ref() == Some(&text) {
            return;
        }
        match bookmarks::parse(&text) {
            Ok(marks) => {
                self.bookmarks = marks;
                self.error = None;
            }
            Err(e) => self.error = Some(format!("{e:#}")),
        }
        self.last_read = Some(text);
        self.clamp_selection();
    }

    /// The bookmarks the query matches, best first; all of them, in file
    /// order, when the query is empty.
    pub fn visible(&mut self) -> Vec<&Bookmark> {
        if self.query.trim().is_empty() {
            return self.bookmarks.iter().collect();
        }
        let pattern = Pattern::parse(&self.query, CaseMatching::Smart, Normalization::Smart);
        let mut buf = Vec::new();
        let mut scored: Vec<(&Bookmark, u32)> = self
            .bookmarks
            .iter()
            .filter_map(|b| {
                let haystack = b.haystack();
                pattern
                    .score(Utf32Str::new(&haystack, &mut buf), &mut self.matcher)
                    .map(|s| (b, s))
            })
            .collect();
        scored.sort_by_key(|(_, score)| std::cmp::Reverse(*score));
        scored.into_iter().map(|(b, _)| b).collect()
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        if ctrl && key.code == KeyCode::Char('c') {
            self.should_quit = true;
            return;
        }
        if let Some(form) = &mut self.form {
            match form.handle_key(key) {
                form::Action::Continue => {}
                form::Action::Cancel => self.form = None,
                form::Action::Submit => self.save_form(),
            }
            return;
        }
        match key.code {
            KeyCode::Char('n') if ctrl => self.open_form(None),
            KeyCode::Char('e') if ctrl => {
                let selected = self.selected;
                if let Some(bookmark) = self.visible().get(selected).map(|b| (*b).clone()) {
                    self.open_form(Some(bookmark));
                }
            }
            KeyCode::Esc if self.query.is_empty() => self.should_quit = true,
            KeyCode::Esc => self.set_query(String::new()),
            KeyCode::Enter => {
                let selected = self.selected;
                if let Some(bookmark) = self.visible().get(selected) {
                    self.chosen = Some((*bookmark).clone());
                    self.should_quit = true;
                }
            }
            KeyCode::Up => self.selected = self.selected.saturating_sub(1),
            KeyCode::Down => {
                self.selected += 1;
                self.clamp_selection();
            }
            KeyCode::Backspace => {
                let mut query = self.query.clone();
                query.pop();
                self.set_query(query);
            }
            KeyCode::Char(c) if !ctrl => {
                let query = format!("{}{c}", self.query);
                self.set_query(query);
            }
            _ => {}
        }
    }

    /// Opens the form for a new bookmark, or to edit one, loading the spaces
    /// it picks from only now.
    fn open_form(&mut self, editing: Option<Bookmark>) {
        let (spaces, error) = match self.config.browser.spaces() {
            Ok(spaces) => (spaces, None),
            Err(e) => (Vec::new(), Some(format!("{e:#}"))),
        };
        self.form = Some(match editing {
            Some(bookmark) => Form::edit(&bookmark, spaces, error),
            None => Form::new(spaces, error),
        });
    }

    fn save_form(&mut self) {
        let Some(form) = &mut self.form else { return };
        let bookmark = form.bookmark();
        let original = form.original.as_deref();
        // an edit may keep its own name, just not take another bookmark's
        let others: Vec<Bookmark> = self
            .bookmarks
            .iter()
            .filter(|b| Some(b.name.as_str()) != original)
            .cloned()
            .collect();
        let path = &self.config.bookmarks;
        let result = validate(&bookmark, &others).and_then(|()| {
            match original {
                Some(original) => bookmarks::replace(path, original, &bookmark),
                None => bookmarks::append(path, &bookmark),
            }
            .map_err(|e| format!("{e:#}"))
        });
        match result {
            Ok(()) => {
                self.form = None;
                self.reload();
            }
            Err(message) => form.error = Some(message),
        }
    }

    fn set_query(&mut self, query: String) {
        self.query = query;
        self.selected = 0;
    }

    fn clamp_selection(&mut self) {
        let count = self.visible().len();
        self.selected = self.selected.min(count.saturating_sub(1));
    }

    pub fn render(&mut self, frame: &mut Frame) {
        let [header, prompt, body, status, footer] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Fill(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .areas(frame.area());

        frame.render_widget(Line::from("booksmart".bold()), header);
        frame.render_widget(
            Line::from(vec![
                Span::from("> ").dim(),
                Span::from(self.query.as_str()),
            ]),
            prompt,
        );

        let selected = self.selected;
        let bookmarks_path = self.config.bookmarks.display().to_string();
        let no_bookmarks = self.bookmarks.is_empty();
        let visible = self.visible();
        if no_bookmarks {
            let hint = Paragraph::new(vec![
                Line::from("no bookmarks yet"),
                Line::from(format!("add some to {bookmarks_path}").dim()),
            ])
            .block(Block::bordered());
            frame.render_widget(hint, body);
        } else {
            let name_width = visible.iter().map(|b| b.name.len()).max().unwrap_or(0);
            let space_width = visible.iter().map(|b| b.space.len()).max().unwrap_or(0);
            let items: Vec<ListItem> = visible
                .iter()
                .map(|b| {
                    ListItem::new(Line::from(vec![
                        Span::from(format!("{:name_width$}  ", b.name)).bold(),
                        Span::from(format!("{:space_width$}  ", b.space)).cyan(),
                        Span::from(b.urls.join(" ")).dim(),
                    ]))
                })
                .collect();
            let list = List::new(items)
                .block(Block::bordered())
                .highlight_style(Style::new().reversed());
            let mut state = ListState::default().with_selected(Some(selected));
            frame.render_stateful_widget(list, body, &mut state);
        }

        if let Some(error) = &self.error {
            let line = Line::from(format!("can't read bookmarks: {error}")).red();
            frame.render_widget(line, status);
        }

        frame.render_widget(
            Line::from("type to filter · ↑↓ select · enter open · ctrl-n new · ctrl-e edit · esc clear/quit")
                .dim(),
            footer,
        );

        if let Some(form) = &self.form {
            form.render(frame);
        }
    }
}

/// Why a bookmark can't be saved, if it can't.
fn validate(bookmark: &Bookmark, existing: &[Bookmark]) -> Result<(), String> {
    if bookmark.name.is_empty() {
        return Err("needs a name".into());
    }
    if bookmark.urls.is_empty() {
        return Err("needs a url".into());
    }
    if bookmark.space.is_empty() {
        return Err("needs a space".into());
    }
    if existing.iter().any(|b| b.name == bookmark.name) {
        return Err(format!("already a bookmark called {}", bookmark.name));
    }
    Ok(())
}
