//! Adequate's spaces, through its `adequate` CLI: `adequate spaces` lists them and
//! `adequate open <space> <url>..` opens tabs, creating the space if Adequate
//! doesn't have it yet.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail};

pub fn default_path() -> PathBuf {
    PathBuf::from("adequate")
}

pub fn spaces(path: &Path) -> Result<Vec<String>> {
    let output = run(path, &["spaces".to_string()])?;
    Ok(output
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(String::from)
        .collect())
}

/// Opens every url as a tab of the space, in one call so Adequate shows the last.
pub fn open(path: &Path, space: &str, urls: &[String]) -> Result<()> {
    let mut args = vec!["open".to_string(), space.to_string()];
    args.extend(urls.iter().cloned());
    run(path, &args)?;
    Ok(())
}

fn run(path: &Path, args: &[String]) -> Result<String> {
    // the TUI may be on screen, so stderr only surfaces as the reason it failed
    let output = Command::new(path)
        .args(args)
        .stdin(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .with_context(|| {
            format!(
                "running {} (is the adequate CLI installed?)",
                path.display()
            )
        })?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        match stderr.trim() {
            "" => bail!("adequate {} failed ({})", args[0], output.status),
            reason => bail!("adequate {} failed: {reason}", args[0]),
        }
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    /// A stand-in `adequate` that logs its arguments and lists two spaces.
    fn fake_adequate(dir: &Path) -> PathBuf {
        let path = dir.join("adequate");
        let log = dir.join("log");
        std::fs::write(
            &path,
            format!(
                "#!/bin/sh\necho \"$@\" >> {}\n[ \"$1\" = spaces ] && printf 'Home\\nWork Stuff\\n\\n'\nexit 0\n",
                log.display()
            ),
        )
        .unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        path
    }

    #[test]
    fn spaces_are_the_lines_adequate_lists() {
        let dir = tempfile::tempdir().unwrap();
        let adequate = fake_adequate(dir.path());
        assert_eq!(spaces(&adequate).unwrap(), ["Home", "Work Stuff"]);
    }

    #[test]
    fn opens_every_url_in_one_call() {
        let dir = tempfile::tempdir().unwrap();
        let adequate = fake_adequate(dir.path());
        open(
            &adequate,
            "Work",
            &[
                "https://a.example.com".into(),
                "https://b.example.com".into(),
            ],
        )
        .unwrap();
        assert_eq!(
            std::fs::read_to_string(dir.path().join("log")).unwrap(),
            "open Work https://a.example.com https://b.example.com\n"
        );
    }

    #[test]
    fn a_failure_reports_adequates_stderr() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("adequate");
        std::fs::write(&path, "#!/bin/sh\necho 'no cli token yet' >&2\nexit 1\n").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        let error = open(&path, "Work", &["https://x.com".into()]).unwrap_err();
        assert!(error.to_string().contains("no cli token yet"), "{error}");
    }

    #[test]
    fn a_missing_cli_says_so() {
        let error = spaces(Path::new("/nonexistent/adequate")).unwrap_err();
        assert!(
            format!("{error:#}").contains("is the adequate CLI installed"),
            "{error:#}"
        );
    }
}
