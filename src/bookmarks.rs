//! The bookmarks file: TOML, one table per bookmark, keyed by its name.
//!
//! ```toml
//! [mail-work]
//! space = "Work"
//! url = "https://mail.example.com"     # or a list of urls that open together
//! tags = ["dev"]                       # optional
//! ```

use std::path::Path;

use anyhow::{Context, Result};

#[derive(Debug, Clone, PartialEq)]
pub struct Bookmark {
    pub name: String,
    pub space: String,
    pub urls: Vec<String>,
    pub tags: Vec<String>,
}

impl Bookmark {
    /// The text a filter query is matched against.
    pub fn haystack(&self) -> String {
        format!(
            "{} {} {} {}",
            self.name,
            self.space,
            self.urls.join(" "),
            self.tags.join(" ")
        )
    }
}

/// Reads the bookmarks file, treating a missing file as empty.
pub fn read(path: &Path) -> Result<String> {
    match std::fs::read_to_string(path) {
        Ok(text) => Ok(text),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
        Err(e) => Err(e).with_context(|| format!("reading {}", path.display())),
    }
}

/// Parses the file's text. A table missing its space or url is skipped, so
/// one half-written bookmark doesn't hide the rest.
pub fn parse(text: &str) -> Result<Vec<Bookmark>> {
    let doc: toml::Table = text.parse()?;
    let mut bookmarks = Vec::new();
    for (name, value) in doc {
        let Some(table) = value.as_table() else {
            continue;
        };
        let Some(space) = table.get("space").and_then(|v| v.as_str()) else {
            continue;
        };
        let urls = strings(table.get("url"));
        if urls.is_empty() {
            continue;
        }
        bookmarks.push(Bookmark {
            name,
            space: space.to_string(),
            urls,
            tags: strings(table.get("tags")),
        });
    }
    Ok(bookmarks)
}

/// A string, or a list of strings, as a list.
fn strings(value: Option<&toml::Value>) -> Vec<String> {
    match value {
        Some(toml::Value::String(s)) => vec![s.clone()],
        Some(toml::Value::Array(items)) => items
            .iter()
            .filter_map(|v| v.as_str())
            .map(str::to_string)
            .collect(),
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn url_may_be_a_string_or_a_list() {
        let marks = parse(
            r#"
            [one]
            space = "Work"
            url = "https://one.example.com"

            [two]
            space = "Home"
            url = ["https://a.example.com", "https://b.example.com"]
            "#,
        )
        .unwrap();
        assert_eq!(marks[0].urls, ["https://one.example.com"]);
        assert_eq!(
            marks[1].urls,
            ["https://a.example.com", "https://b.example.com"]
        );
    }

    #[test]
    fn tags_are_optional() {
        let marks = parse("[one]\nspace = 'Work'\nurl = 'x.com'\n").unwrap();
        assert!(marks[0].tags.is_empty());
        let marks = parse("[one]\nspace = 'Work'\nurl = 'x.com'\ntags = ['a', 'b']\n").unwrap();
        assert_eq!(marks[0].tags, ["a", "b"]);
    }

    #[test]
    fn tables_missing_space_or_url_are_skipped() {
        let marks = parse(
            r#"
            [no-space]
            url = "https://x.example.com"

            [no-url]
            space = "Work"

            [fine]
            space = "Work"
            url = "https://fine.example.com"
            "#,
        )
        .unwrap();
        let names: Vec<_> = marks.iter().map(|m| m.name.as_str()).collect();
        assert_eq!(names, ["fine"]);
    }

    #[test]
    fn file_order_is_kept() {
        let marks = parse("[zed]\nspace='a'\nurl='z'\n[alpha]\nspace='a'\nurl='a'\n").unwrap();
        let names: Vec<_> = marks.iter().map(|m| m.name.as_str()).collect();
        assert_eq!(names, ["zed", "alpha"]);
    }

    #[test]
    fn invalid_toml_is_an_error() {
        assert!(parse("[broken").is_err());
    }

    #[test]
    fn a_missing_file_reads_as_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(read(&dir.path().join("nope.toml")).unwrap(), "");
    }
}
