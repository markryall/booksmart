//! Opening urls in a browser space. Each kind of browser is configured under
//! `[browser]` with a `type`; the default opens urls in the system's default
//! browser and ignores spaces.

use std::path::PathBuf;
use std::process::Command;

use anyhow::{Context, Result, bail};
use serde::Deserialize;

use crate::crest;

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase", deny_unknown_fields)]
pub enum Browser {
    /// Runs `command` once per url, with `{space}` and `{url}` filled in.
    Command {
        command: Vec<String>,
        /// The spaces to offer when creating a bookmark.
        #[serde(default)]
        spaces: Vec<String>,
    },
    /// Crest's spaces, through the MCP server bundled with Crest.
    Crest {
        #[serde(default = "crest::default_path")]
        path: PathBuf,
    },
}

impl Default for Browser {
    fn default() -> Browser {
        Browser::Command {
            command: vec!["open".into(), "{url}".into()],
            spaces: Vec::new(),
        }
    }
}

impl Browser {
    /// Opens every url, in order, in the space.
    pub fn open(&self, space: &str, urls: &[String]) -> Result<()> {
        let urls: Vec<String> = urls.iter().map(|u| with_scheme(u)).collect();
        match self {
            Browser::Command { command, .. } => {
                for url in &urls {
                    run(command, space, url)?;
                }
                Ok(())
            }
            Browser::Crest { path } => crest::Crest::connect(path)?.open(space, &urls),
        }
    }
}

fn run(command: &[String], space: &str, url: &str) -> Result<()> {
    let Some((program, args)) = command.split_first() else {
        bail!("the browser command is empty");
    };
    let fill = |arg: &String| arg.replace("{space}", space).replace("{url}", url);
    let status = Command::new(fill(program))
        .args(args.iter().map(fill))
        .status()
        .with_context(|| format!("running {program}"))?;
    if !status.success() {
        bail!("{program} failed ({status}) opening {url}");
    }
    Ok(())
}

/// Bookmarks may leave off the scheme; browsers need it.
pub fn with_scheme(url: &str) -> String {
    let has_scheme = url.split_once("://").is_some_and(|(scheme, _)| {
        !scheme.is_empty()
            && scheme
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || "+-.".contains(c))
    });
    if has_scheme {
        url.to_string()
    } else {
        format!("https://{url}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urls_without_a_scheme_get_https() {
        assert_eq!(with_scheme("example.com/a"), "https://example.com/a");
        assert_eq!(with_scheme("https://example.com"), "https://example.com");
        assert_eq!(with_scheme("http://example.com"), "http://example.com");
        assert_eq!(
            with_scheme("example.com/?next=https://x"),
            "https://example.com/?next=https://x"
        );
    }

    #[test]
    fn the_default_opens_in_the_system_browser() {
        assert_eq!(
            Browser::default(),
            Browser::Command {
                command: vec!["open".into(), "{url}".into()],
                spaces: vec![]
            }
        );
    }

    #[test]
    fn command_fills_space_and_url() {
        let dir = tempfile::tempdir().unwrap();
        let log = dir.path().join("log");
        let browser = Browser::Command {
            command: vec![
                "sh".into(),
                "-c".into(),
                format!("echo \"$1 $2\" >> {}", log.display()),
                "sh".into(),
                "{space}".into(),
                "{url}".into(),
            ],
            spaces: vec![],
        };
        browser
            .open(
                "Work",
                &["a.example.com".into(), "https://b.example.com".into()],
            )
            .unwrap();
        assert_eq!(
            std::fs::read_to_string(log).unwrap(),
            "Work https://a.example.com\nWork https://b.example.com\n"
        );
    }

    #[test]
    fn a_failing_command_is_an_error() {
        let browser = Browser::Command {
            command: vec!["false".into()],
            spaces: vec![],
        };
        assert!(browser.open("Work", &["x.com".into()]).is_err());
    }
}
