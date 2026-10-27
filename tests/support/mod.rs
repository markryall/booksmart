//! Drives the real booksmart binary in a pseudo-terminal and reads the screen
//! back through a vt100 parser, so feature specs see what a person would see.

#![allow(dead_code)]

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use portable_pty::{Child, CommandBuilder, MasterPty, PtySize, native_pty_system};
use tempfile::TempDir;

const ROWS: u16 = 30;
const COLS: u16 = 100;
const TIMEOUT: Duration = Duration::from_secs(5);

/// A scratch home for one scenario: a config file and the bookmarks it points at.
pub struct World {
    pub dir: TempDir,
}

impl World {
    pub fn new() -> Self {
        let world = World {
            dir: TempDir::new().unwrap(),
        };
        world.write_config(&format!("bookmarks = {:?}\n", world.bookmarks_path()));
        world
    }

    pub fn path(&self, name: &str) -> PathBuf {
        self.dir.path().join(name)
    }

    pub fn config_path(&self) -> PathBuf {
        self.path("config.toml")
    }

    pub fn bookmarks_path(&self) -> PathBuf {
        self.path("bookmarks.toml")
    }

    pub fn write_config(&self, contents: &str) {
        std::fs::write(self.config_path(), contents).unwrap();
    }

    pub fn write_bookmarks(&self, contents: &str) {
        std::fs::write(self.bookmarks_path(), contents).unwrap();
    }

    pub fn launch(&self) -> Session {
        Session::spawn(&self.config_path(), self.dir.path())
    }
}

/// Polls until `check` passes, panicking with `what` if it never does.
pub fn eventually(what: &str, mut check: impl FnMut() -> bool) {
    let start = Instant::now();
    while start.elapsed() < TIMEOUT {
        if check() {
            return;
        }
        thread::sleep(Duration::from_millis(20));
    }
    panic!("never happened: {what}");
}

pub struct Session {
    screen: Arc<Mutex<vt100::Parser>>,
    writer: Box<dyn Write + Send>,
    child: Box<dyn Child + Send + Sync>,
    _master: Box<dyn MasterPty + Send>,
}

impl Session {
    fn spawn(config: &Path, home: &Path) -> Self {
        let pty = native_pty_system()
            .openpty(PtySize {
                rows: ROWS,
                cols: COLS,
                pixel_width: 0,
                pixel_height: 0,
            })
            .unwrap();
        let mut cmd = CommandBuilder::new(env!("CARGO_BIN_EXE_booksmart"));
        cmd.env("BOOKSMART_CONFIG", config);
        cmd.env("HOME", home);
        cmd.env("TERM", "xterm-256color");
        let child = pty.slave.spawn_command(cmd).unwrap();
        drop(pty.slave);

        let screen = Arc::new(Mutex::new(vt100::Parser::new(
            ROWS.try_into().unwrap(),
            COLS.try_into().unwrap(),
            0,
        )));
        let mut reader = pty.master.try_clone_reader().unwrap();
        let feed = Arc::clone(&screen);
        thread::spawn(move || {
            let mut buf = [0u8; 4096];
            while let Ok(n) = reader.read(&mut buf) {
                if n == 0 {
                    break;
                }
                feed.lock().unwrap().process(&buf[..n]);
            }
        });
        let writer = pty.master.take_writer().unwrap();
        Session {
            screen,
            writer,
            child,
            _master: pty.master,
        }
    }

    pub fn contents(&self) -> String {
        self.screen.lock().unwrap().screen().contents()
    }

    /// Waits until `text` is on screen, panicking with the screen if it never shows.
    pub fn expect(&self, text: &str) {
        let start = Instant::now();
        while start.elapsed() < TIMEOUT {
            if self.contents().contains(text) {
                return;
            }
            thread::sleep(Duration::from_millis(20));
        }
        panic!("never saw {text:?} on screen:\n{}", self.contents());
    }

    /// Waits until `text` is gone from the screen.
    pub fn expect_gone(&self, text: &str) {
        let start = Instant::now();
        while start.elapsed() < TIMEOUT {
            if !self.contents().contains(text) {
                return;
            }
            thread::sleep(Duration::from_millis(20));
        }
        panic!("{text:?} stayed on screen:\n{}", self.contents());
    }

    pub fn type_text(&mut self, text: &str) {
        self.writer.write_all(text.as_bytes()).unwrap();
        self.writer.flush().unwrap();
    }

    pub fn press(&mut self, key: Key) {
        let ctrl;
        let bytes: &[u8] = match key {
            Key::Ctrl(c) => {
                ctrl = [c.to_ascii_lowercase() as u8 & 0x1f];
                &ctrl
            }
            Key::Enter => b"\r",
            Key::Esc => b"\x1b",
            Key::Backspace => b"\x7f",
            Key::Tab => b"\t",
            Key::BackTab => b"\x1b[Z",
            Key::Up => b"\x1b[A",
            Key::Down => b"\x1b[B",
        };
        self.writer.write_all(bytes).unwrap();
        self.writer.flush().unwrap();
    }

    /// Whether booksmart is still running.
    pub fn running(&mut self) -> bool {
        self.child.try_wait().unwrap().is_none()
    }

    /// Waits for the process to exit and returns whether it succeeded.
    pub fn wait_exit(&mut self) -> bool {
        let start = Instant::now();
        while start.elapsed() < TIMEOUT {
            if let Some(status) = self.child.try_wait().unwrap() {
                return status.success();
            }
            thread::sleep(Duration::from_millis(20));
        }
        panic!("booksmart never exited; screen:\n{}", self.contents());
    }

    /// Whether the alternate screen was left, i.e. the terminal was restored.
    pub fn restored(&self) -> bool {
        !self.screen.lock().unwrap().screen().alternate_screen()
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        let _ = self.child.kill();
    }
}

#[derive(Clone, Copy)]
pub enum Key {
    Ctrl(char),
    BackTab,
    Enter,
    Esc,
    Backspace,
    Tab,
    Up,
    Down,
}
