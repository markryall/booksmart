//! Crest's spaces, reached through `crest-browser mcp`, the MCP server bundled
//! with Crest. Only the spaces allowed in Crest's Settings › Agents are visible.

use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

use anyhow::{Context, Result, anyhow, bail};
use serde::Deserialize;
use serde_json::{Value, json};

pub fn default_path() -> PathBuf {
    PathBuf::from("/Applications/Crest.app/Contents/Helpers/crest-browser")
}

#[derive(Debug, Clone, Deserialize)]
pub struct Space {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub locked: bool,
}

/// A minimal MCP client: JSON-RPC over newline-delimited stdio.
pub struct Client<R, W> {
    reader: R,
    writer: W,
    next_id: u64,
}

impl<R: BufRead, W: Write> Client<R, W> {
    pub fn new(reader: R, writer: W) -> Result<Client<R, W>> {
        let mut client = Client {
            reader,
            writer,
            next_id: 0,
        };
        client.request(
            "initialize",
            json!({
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": {"name": "booksmart", "version": env!("CARGO_PKG_VERSION")},
            }),
        )?;
        client.send(&json!({"jsonrpc": "2.0", "method": "notifications/initialized"}))?;
        // so the tabs booksmart opens carry its badge in Crest
        client.call("identify", json!({"name": "booksmart", "color": "#FF69B4"}))?;
        Ok(client)
    }

    fn send(&mut self, message: &Value) -> Result<()> {
        writeln!(self.writer, "{message}")?;
        self.writer.flush()?;
        Ok(())
    }

    fn request(&mut self, method: &str, params: Value) -> Result<Value> {
        self.next_id += 1;
        let id = self.next_id;
        self.send(&json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}))?;
        let mut line = String::new();
        loop {
            line.clear();
            if self.reader.read_line(&mut line)? == 0 {
                bail!("crest closed the connection during {method}");
            }
            let message: Value = serde_json::from_str(&line)
                .with_context(|| format!("reading crest's reply to {method}"))?;
            if message.get("id") != Some(&json!(id)) {
                continue;
            }
            if let Some(error) = message.get("error") {
                bail!(
                    "crest {method}: {}",
                    error["message"].as_str().unwrap_or("failed")
                );
            }
            return Ok(message["result"].clone());
        }
    }

    /// Calls a tool and answers its text content, parsed as JSON when it is.
    pub fn call(&mut self, tool: &str, arguments: Value) -> Result<Value> {
        let result = self.request("tools/call", json!({"name": tool, "arguments": arguments}))?;
        let text: String = result["content"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|c| c["text"].as_str())
            .collect();
        if result["isError"].as_bool() == Some(true) {
            bail!("crest {tool}: {text}");
        }
        Ok(serde_json::from_str(&text).unwrap_or(Value::String(text)))
    }

    pub fn spaces(&mut self) -> Result<Vec<Space>> {
        let listed = self.call("list_spaces", json!({}))?;
        Ok(serde_json::from_value(listed["spaces"].clone())?)
    }

    /// Opens the urls as tabs of the space matching `query`, bringing the
    /// last one forward.
    pub fn open(&mut self, query: &str, urls: &[String]) -> Result<()> {
        let spaces = self.spaces()?;
        let space = find_space(&spaces, query)?;
        for (i, url) in urls.iter().enumerate() {
            let show = i + 1 == urls.len();
            self.call(
                "open_tab",
                json!({"space": space.id, "url": url, "show": show}),
            )?;
        }
        Ok(())
    }
}

/// The space named `query`, ignoring case and punctuation, preferring an
/// exact match over one that merely contains it.
pub fn find_space<'a>(spaces: &'a [Space], query: &str) -> Result<&'a Space> {
    let q = loose(query);
    let space = spaces
        .iter()
        .find(|s| loose(&s.name) == q)
        .or_else(|| {
            spaces
                .iter()
                .find(|s| !q.is_empty() && loose(&s.name).contains(&q))
        })
        .ok_or_else(|| {
            let names: Vec<_> = spaces.iter().map(|s| s.name.as_str()).collect();
            anyhow!(
                "no crest space matches {query:?} (spaces: {})",
                names.join(", ")
            )
        })?;
    if space.locked {
        bail!(
            "crest space {} is locked; unlock it in Crest first",
            space.name
        );
    }
    Ok(space)
}

fn loose(name: &str) -> String {
    name.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

/// The MCP server as a child process.
pub struct Crest {
    child: Child,
    client: Client<BufReader<std::process::ChildStdout>, std::process::ChildStdin>,
}

impl Crest {
    pub fn connect(path: &Path) -> Result<Crest> {
        let mut child = Command::new(path)
            .arg("mcp")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .with_context(|| format!("starting {} (is Crest installed?)", path.display()))?;
        let reader = BufReader::new(child.stdout.take().expect("piped stdout"));
        let writer = child.stdin.take().expect("piped stdin");
        let client = Client::new(reader, writer)?;
        Ok(Crest { child, client })
    }

    pub fn open(mut self, space: &str, urls: &[String]) -> Result<()> {
        self.client.open(space, urls)
    }

    pub fn spaces(mut self) -> Result<Vec<Space>> {
        self.client.spaces()
    }
}

impl Drop for Crest {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{PipeReader, PipeWriter, pipe};
    use std::sync::{Arc, Mutex};
    use std::thread;

    type FakeClient = Client<BufReader<PipeReader>, PipeWriter>;
    type Calls = Arc<Mutex<Vec<Value>>>;

    /// Answers like crest-browser does, and records every tool call it gets.
    fn fake_crest(spaces: Value) -> (FakeClient, Calls) {
        let (server_in, client_out) = pipe().unwrap();
        let (client_in, server_out) = pipe().unwrap();
        let calls = Arc::new(Mutex::new(Vec::new()));
        let seen = Arc::clone(&calls);
        thread::spawn(move || {
            let mut out = server_out;
            for line in BufReader::new(server_in).lines() {
                let message: Value = serde_json::from_str(&line.unwrap()).unwrap();
                let Some(id) = message.get("id").cloned() else {
                    continue;
                };
                let result = match message["method"].as_str().unwrap() {
                    "tools/call" => {
                        seen.lock().unwrap().push(message["params"].clone());
                        let text = match message["params"]["name"].as_str().unwrap() {
                            "list_spaces" => json!({"spaces": spaces}).to_string(),
                            _ => "{}".to_string(),
                        };
                        json!({"content": [{"type": "text", "text": text}], "isError": false})
                    }
                    _ => json!({}),
                };
                writeln!(
                    out,
                    "{}",
                    json!({"jsonrpc": "2.0", "id": id, "result": result})
                )
                .unwrap();
            }
        });
        let client = Client::new(BufReader::new(client_in), client_out).unwrap();
        (client, calls)
    }

    fn spaces() -> Value {
        json!([
            {"id": "s-home", "name": "home", "locked": false},
            {"id": "s-side", "name": "side project", "locked": false},
            {"id": "s-work-test", "name": "work-test", "locked": false},
            {"id": "s-work", "name": "Work", "locked": false},
            {"id": "s-vault", "name": "Vault", "locked": true},
        ])
    }

    fn opened(calls: &Mutex<Vec<Value>>) -> Vec<Value> {
        calls
            .lock()
            .unwrap()
            .iter()
            .filter(|c| c["name"] == "open_tab")
            .map(|c| c["arguments"].clone())
            .collect()
    }

    #[test]
    fn opens_in_the_space_matched_loosely_with_show_on_the_last_url() {
        let (mut client, calls) = fake_crest(spaces());
        client
            .open(
                "Side-Project",
                &[
                    "https://a.example.com".into(),
                    "https://b.example.com".into(),
                ],
            )
            .unwrap();
        assert_eq!(
            opened(&calls),
            [
                json!({"space": "s-side", "url": "https://a.example.com", "show": false}),
                json!({"space": "s-side", "url": "https://b.example.com", "show": true}),
            ]
        );
    }

    #[test]
    fn identifies_itself_before_anything_else() {
        let (mut client, calls) = fake_crest(spaces());
        client
            .open("home", &["https://a.example.com".into()])
            .unwrap();
        assert_eq!(calls.lock().unwrap()[0]["name"], "identify");
    }

    #[test]
    fn an_exact_match_beats_one_that_contains_the_query() {
        let (mut client, calls) = fake_crest(spaces());
        client
            .open("work", &["https://a.example.com".into()])
            .unwrap();
        assert_eq!(opened(&calls)[0]["space"], "s-work");
    }

    #[test]
    fn an_unknown_space_names_the_known_ones() {
        let (mut client, _) = fake_crest(spaces());
        let error = client
            .open("nope", &["https://a.example.com".into()])
            .unwrap_err();
        assert!(error.to_string().contains("home, side project"), "{error}");
    }

    #[test]
    fn spaces_come_from_list_spaces() {
        let (mut client, _) = fake_crest(spaces());
        let names: Vec<_> = client
            .spaces()
            .unwrap()
            .into_iter()
            .map(|s| s.name)
            .collect();
        assert_eq!(
            names,
            ["home", "side project", "work-test", "Work", "Vault"]
        );
    }

    #[test]
    fn a_locked_space_is_refused() {
        let (mut client, calls) = fake_crest(spaces());
        assert!(
            client
                .open("vault", &["https://a.example.com".into()])
                .is_err()
        );
        assert!(opened(&calls).is_empty());
    }
}
