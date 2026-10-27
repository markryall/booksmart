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

/// The bookmark as a TOML table of its own.
pub fn to_toml(bookmark: &Bookmark) -> String {
    use toml_edit::{Array, DocumentMut, Item, Table, value};
    let mut table = Table::new();
    table["space"] = value(bookmark.space.as_str());
    table["url"] = match bookmark.urls.as_slice() {
        [url] => value(url.as_str()),
        urls => value(urls.iter().map(String::as_str).collect::<Array>()),
    };
    if !bookmark.tags.is_empty() {
        table["tags"] = value(bookmark.tags.iter().map(String::as_str).collect::<Array>());
    }
    let mut doc = DocumentMut::new();
    doc.insert(&bookmark.name, Item::Table(table));
    doc.to_string()
}

/// Adds the bookmark to the end of the file, writing only the new bytes so
/// everything already there, comments included, stays exactly as it was.
pub fn append(path: &Path, bookmark: &Bookmark) -> Result<()> {
    use std::io::Write;
    let existing = read(path)?;
    let separator = match existing.as_str() {
        "" => "",
        text if text.ends_with("\n\n") => "",
        text if text.ends_with('\n') => "\n",
        _ => "\n\n",
    };
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .with_context(|| format!("opening {}", path.display()))?;
    write!(file, "{separator}{}", to_toml(bookmark))
        .with_context(|| format!("writing {}", path.display()))?;
    Ok(())
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

    fn calendar() -> Bookmark {
        Bookmark {
            name: "calendar".into(),
            space: "Home".into(),
            urls: vec!["calendar.example.com".into()],
            tags: vec!["dev".into()],
        }
    }

    #[test]
    fn appending_keeps_existing_bytes_exactly() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("marks.toml");
        let original = "# mine, keep this\n[zed]   # odd spacing too\nspace='a'\nurl = 'z'\n";
        std::fs::write(&path, original).unwrap();
        append(&path, &calendar()).unwrap();
        let after = std::fs::read_to_string(&path).unwrap();
        assert!(after.starts_with(original), "{after}");
        assert_eq!(parse(&after).unwrap()[1], calendar());
    }

    #[test]
    fn appending_to_a_file_without_a_trailing_newline_still_parses() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("marks.toml");
        std::fs::write(&path, "[zed]\nspace='a'\nurl='z'").unwrap();
        append(&path, &calendar()).unwrap();
        let names: Vec<_> = parse(&std::fs::read_to_string(&path).unwrap())
            .unwrap()
            .into_iter()
            .map(|b| b.name)
            .collect();
        assert_eq!(names, ["zed", "calendar"]);
    }

    #[test]
    fn appending_creates_a_missing_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("marks.toml");
        append(&path, &calendar()).unwrap();
        assert_eq!(
            parse(&std::fs::read_to_string(&path).unwrap()).unwrap(),
            [calendar()]
        );
    }

    #[test]
    fn multiple_urls_round_trip_as_a_list_and_odd_names_are_quoted() {
        let bookmark = Bookmark {
            name: "two words".into(),
            space: "Work".into(),
            urls: vec!["a.example.com".into(), "b.example.com".into()],
            tags: vec![],
        };
        let text = to_toml(&bookmark);
        assert!(text.contains(r#"["two words"]"#), "{text}");
        assert_eq!(parse(&text).unwrap(), [bookmark]);
    }

    #[test]
    fn a_missing_file_reads_as_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(read(&dir.path().join("nope.toml")).unwrap(), "");
    }
}
