//! Where booksmart reads its settings: `$BOOKSMART_CONFIG`, else
//! `~/.config/booksmart/config.toml`. Every setting has a default, so a
//! missing config file is fine.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::browser::Browser;

#[derive(Debug, Clone, PartialEq)]
pub struct Config {
    pub bookmarks: PathBuf,
    pub browser: Browser,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawConfig {
    bookmarks: Option<String>,
    browser: Option<Browser>,
}

impl Config {
    /// Loads the config from its usual place, falling back to defaults when
    /// there is no file.
    pub fn load() -> Result<Config> {
        let home = home();
        let path = match std::env::var_os("BOOKSMART_CONFIG") {
            Some(path) => PathBuf::from(path),
            None => home.join(".config/booksmart/config.toml"),
        };
        let text = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
            Err(e) => return Err(e).with_context(|| format!("reading {}", path.display())),
        };
        Config::parse(&text, &home).with_context(|| format!("in {}", path.display()))
    }

    pub fn parse(text: &str, home: &Path) -> Result<Config> {
        let raw: RawConfig = toml::from_str(text)?;
        let bookmarks = match raw.bookmarks {
            Some(path) => expand(&path, home),
            None => home.join(".config/booksmart/bookmarks.toml"),
        };
        Ok(Config {
            bookmarks,
            browser: raw.browser.unwrap_or_default(),
        })
    }
}

fn home() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_default()
}

fn expand(path: &str, home: &Path) -> PathBuf {
    match path.strip_prefix("~/") {
        Some(rest) => home.join(rest),
        None if path == "~" => home.to_path_buf(),
        None => PathBuf::from(path),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bookmarks_path_defaults_when_omitted() {
        let config = Config::parse("", Path::new("/home/me")).unwrap();
        assert_eq!(
            config.bookmarks,
            Path::new("/home/me/.config/booksmart/bookmarks.toml")
        );
    }

    #[test]
    fn bookmarks_path_expands_tilde() {
        let config = Config::parse(
            r#"bookmarks = "~/.browser-bookmarks.toml""#,
            Path::new("/home/me"),
        )
        .unwrap();
        assert_eq!(
            config.bookmarks,
            Path::new("/home/me/.browser-bookmarks.toml")
        );
    }

    #[test]
    fn absolute_bookmarks_path_is_kept() {
        let config =
            Config::parse(r#"bookmarks = "/tmp/marks.toml""#, Path::new("/home/me")).unwrap();
        assert_eq!(config.bookmarks, Path::new("/tmp/marks.toml"));
    }

    #[test]
    fn browser_defaults_to_the_system_browser() {
        let config = Config::parse("", Path::new("/home/me")).unwrap();
        assert_eq!(config.browser, Browser::default());
    }

    #[test]
    fn crest_browser_defaults_its_path() {
        let config = Config::parse("[browser]\ntype = 'crest'\n", Path::new("/home/me")).unwrap();
        assert_eq!(
            config.browser,
            Browser::Crest {
                path: crate::crest::default_path()
            }
        );
    }

    #[test]
    fn unknown_browser_settings_are_rejected() {
        assert!(
            Config::parse(
                "[browser]\ntype = 'crest'\npth = 'x'\n",
                Path::new("/home/me")
            )
            .is_err()
        );
    }

    #[test]
    fn unknown_settings_are_rejected() {
        assert!(Config::parse("bookmark = 'typo'", Path::new("/home/me")).is_err());
    }
}
