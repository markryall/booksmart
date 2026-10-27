//! Urls with placeholders: `{1}`, `{2}`.. take words in order, and `{*}`
//! takes whatever words are left, joined with spaces. Values are url-escaped.

/// Whether any of the urls has a placeholder to fill.
pub fn is_template(urls: &[String]) -> bool {
    urls.iter()
        .any(|url| highest_positional(url) > 0 || url.contains("{*}"))
}

/// The url with its placeholders filled from the words of `input`, or which
/// placeholder has no value.
pub fn fill(url: &str, input: &str) -> Result<String, String> {
    let words: Vec<&str> = input.split_whitespace().collect();
    let positional = highest_positional(url);
    let mut filled = url.to_string();
    for n in 1..=positional {
        let placeholder = format!("{{{n}}}");
        if !filled.contains(&placeholder) {
            continue;
        }
        let word = words
            .get(n - 1)
            .ok_or_else(|| format!("needs a value for {placeholder}"))?;
        filled = filled.replace(&placeholder, &escape(word));
    }
    if filled.contains("{*}") {
        let rest = words.get(positional..).unwrap_or_default();
        if rest.is_empty() {
            return Err("needs a value for {*}".into());
        }
        filled = filled.replace("{*}", &escape(&rest.join(" ")));
    }
    Ok(filled)
}

/// Fills every url from the same input.
pub fn fill_all(urls: &[String], input: &str) -> Result<Vec<String>, String> {
    urls.iter().map(|url| fill(url, input)).collect()
}

/// The highest `{n}` in the url, or 0 when there are none.
fn highest_positional(url: &str) -> usize {
    url.split('{')
        .skip(1)
        .filter_map(|part| part.split_once('}'))
        .filter_map(|(n, _)| n.parse::<usize>().ok())
        .max()
        .unwrap_or(0)
}

/// Percent-escapes everything but the unreserved characters.
fn escape(value: &str) -> String {
    value
        .bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                (b as char).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn positional_placeholders_take_words_in_order() {
        assert_eq!(
            fill("x.com/{1}/orders/{2}", "42 7").unwrap(),
            "x.com/42/orders/7"
        );
    }

    #[test]
    fn a_positional_placeholder_may_repeat() {
        assert_eq!(
            fill("x.com/{1}?back={1}", "42").unwrap(),
            "x.com/42?back=42"
        );
    }

    #[test]
    fn star_takes_the_rest_joined_with_spaces() {
        assert_eq!(
            fill("x.com/{1}?q={*}", "42 sci who").unwrap(),
            "x.com/42?q=sci%20who"
        );
    }

    #[test]
    fn values_are_escaped() {
        assert_eq!(
            fill("x.com/?q={*}", "a&b=c/d é").unwrap(),
            "x.com/?q=a%26b%3Dc%2Fd%20%C3%A9"
        );
    }

    #[test]
    fn a_missing_value_names_its_placeholder() {
        assert_eq!(
            fill("x.com/{1}/{2}", "42"),
            Err("needs a value for {2}".into())
        );
        assert_eq!(
            fill("x.com/{1}?q={*}", "42"),
            Err("needs a value for {*}".into())
        );
        assert_eq!(fill("x.com/{1}", ""), Err("needs a value for {1}".into()));
    }

    #[test]
    fn urls_without_placeholders_pass_through() {
        assert!(!is_template(&["x.com/{curly}/a".into()]));
        assert_eq!(
            fill("x.com/{curly}/a", "ignored").unwrap(),
            "x.com/{curly}/a"
        );
    }

    #[test]
    fn templates_are_spotted_in_any_url() {
        assert!(is_template(&["a.com".into(), "b.com/{1}".into()]));
        assert!(is_template(&["b.com/?q={*}".into()]));
    }
}
