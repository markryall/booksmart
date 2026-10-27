//! What the command line asks for: the TUI, a flag, or opening one bookmark
//! straight away.

use anyhow::{Result, bail};
use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Matcher, Utf32Str};

use crate::bookmarks::Bookmark;
use crate::template;

#[derive(Debug, PartialEq)]
pub enum Invocation {
    Tui,
    Help,
    Version,
    /// `booksmart <name> [values..]`
    Open {
        name: String,
        values: Vec<String>,
    },
}

pub fn parse(args: impl IntoIterator<Item = String>) -> Result<Invocation> {
    let mut args = args.into_iter();
    let Some(first) = args.next() else {
        return Ok(Invocation::Tui);
    };
    Ok(match first.as_str() {
        "-h" | "--help" => Invocation::Help,
        "-V" | "--version" => Invocation::Version,
        flag if flag.starts_with('-') => bail!("unknown argument {flag} (try --help)"),
        _ => Invocation::Open {
            name: first,
            values: args.collect(),
        },
    })
}

/// The bookmark called `name`, with its placeholders filled from `values`
/// the way the prompt fills them: words in order for `{1}` `{2}`.., the
/// rest for `{*}`.
pub fn resolve(bookmarks: &[Bookmark], name: &str, values: &[String]) -> Result<Bookmark> {
    let Some(bookmark) = bookmarks.iter().find(|b| b.name == name) else {
        let close = closest(bookmarks, name);
        if close.is_empty() {
            bail!("no bookmark called {name}");
        }
        bail!(
            "no bookmark called {name}; did you mean {}?",
            close.join(", ")
        );
    };
    if !template::is_template(&bookmark.urls) {
        if !values.is_empty() {
            bail!("{name} takes no values");
        }
        return Ok(bookmark.clone());
    }
    match template::fill_all(&bookmark.urls, &values.join(" ")) {
        Ok(urls) => Ok(Bookmark {
            urls,
            ..bookmark.clone()
        }),
        Err(message) => bail!("{name} {message}"),
    }
}

/// Up to three bookmark names that fuzzily match `name`, best first.
fn closest(bookmarks: &[Bookmark], name: &str) -> Vec<String> {
    let pattern = Pattern::parse(name, CaseMatching::Smart, Normalization::Smart);
    let mut matcher = Matcher::default();
    let mut buf = Vec::new();
    let mut scored: Vec<(&str, u32)> = bookmarks
        .iter()
        .filter_map(|b| {
            pattern
                .score(Utf32Str::new(&b.name, &mut buf), &mut matcher)
                .map(|score| (b.name.as_str(), score))
        })
        .collect();
    scored.sort_by_key(|(_, score)| std::cmp::Reverse(*score));
    scored
        .into_iter()
        .take(3)
        .map(|(n, _)| n.to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn no_arguments_runs_the_tui() {
        assert_eq!(parse(args(&[])).unwrap(), Invocation::Tui);
    }

    #[test]
    fn help_and_version_still_answer() {
        assert_eq!(parse(args(&["--help"])).unwrap(), Invocation::Help);
        assert_eq!(parse(args(&["-h"])).unwrap(), Invocation::Help);
        assert_eq!(parse(args(&["--version"])).unwrap(), Invocation::Version);
        assert_eq!(parse(args(&["-V"])).unwrap(), Invocation::Version);
    }

    #[test]
    fn a_name_starting_with_a_dash_is_an_unknown_argument() {
        let error = parse(args(&["--nope"])).unwrap_err();
        assert!(error.to_string().contains("unknown argument --nope"));
    }

    #[test]
    fn a_name_and_values_open() {
        assert_eq!(
            parse(args(&["supplier", "42", "7"])).unwrap(),
            Invocation::Open {
                name: "supplier".into(),
                values: args(&["42", "7"]),
            }
        );
    }

    fn marks() -> Vec<Bookmark> {
        crate::bookmarks::parse(
            "[mail-work]\nspace='Work'\nurl='mail.example.com'\n\
             [mail-home]\nspace='Home'\nurl='mail.example.com/u/1'\n\
             [order]\nspace='Work'\nurl='shop.example.com/orders/{1}'\n",
        )
        .unwrap()
    }

    #[test]
    fn values_fill_a_template() {
        let order = resolve(&marks(), "order", &args(&["42"])).unwrap();
        assert_eq!(order.urls, ["shop.example.com/orders/42"]);
    }

    #[test]
    fn suggestions_are_fuzzy_matches_on_the_name() {
        let error = resolve(&marks(), "mail", &[]).unwrap_err().to_string();
        assert!(
            error.contains("mail-work") && error.contains("mail-home"),
            "{error}"
        );
        assert!(!error.contains("order"), "{error}");
    }

    #[test]
    fn nothing_close_just_says_so() {
        let error = resolve(&marks(), "zzz", &[]).unwrap_err().to_string();
        assert_eq!(error, "no bookmark called zzz");
    }
}
