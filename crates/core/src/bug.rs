//! Bug drafts: a failed job's `failure.json` -> a scrubbed Markdown report for a person to approve.
//!
//! Nothing here files anything. Filing (`gh issue create`) happens only after the user approves
//! the specific draft, following `.cursor/skills/artcraft-bug-report`.

use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::{Error, Result, paths};

#[derive(Debug, Clone, serde::Serialize)]
pub struct Draft {
    pub repo: String,
    pub title: String,
    pub body: String,
    pub path: PathBuf,
}

/// Replace the home folder and user name so a draft carries no personal paths.
pub fn scrub(text: &str) -> String {
    let mut out = text.to_string();
    if let Some(home) = paths::home() {
        let h = home.display().to_string();
        if h.len() > 1 {
            out = out.replace(&h, "~");
        }
    }
    for var in ["USER", "LOGNAME"] {
        if let Ok(user) = std::env::var(var)
            && user.len() >= 3
        {
            out = out.replace(&user, "<user>");
        }
    }
    out
}

fn s<'a>(v: &'a Value, key: &str) -> &'a str {
    v.get(key).and_then(Value::as_str).unwrap_or("")
}

pub fn draft(job_dir: &Path) -> Result<Draft> {
    let failure_path = job_dir.join("failure.json");
    let text = std::fs::read_to_string(&failure_path).map_err(|e| Error::io(&failure_path, e))?;
    let f: Value = serde_json::from_str(&text)?;
    let app = s(&f, "app");
    if app.is_empty() {
        return Err(Error::Other(format!("{} names no app", failure_path.display())));
    }
    let command = f
        .get("command")
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(Value::as_str).map(shell_quote).collect::<Vec<_>>().join(" "))
        .unwrap_or_default();
    let what = if f.get("timed_out").and_then(Value::as_bool) == Some(true) {
        "did not finish (stopped by timeout)".to_string()
    } else {
        format!("exited with {}", f.get("exit_code").map(Value::to_string).unwrap_or_else(|| "no code".into()))
    };
    let title = format!("{app}-cli {}: `{}` step {what}", s(&f, "app_version"), s(&f, "step"));
    let body = format!(
        "## Summary\n\n`{app}-cli` {what} while running a scripted workflow step.\n\n\
         ## Environment\n\n- App: {app} {ver}\n- OS: {os} ({arch})\n\n\
         ## Steps to reproduce\n\n```sh\n{command}\n```\n\n\
         ## Expected\n\nThe command completes and writes its output file.\n\n\
         ## Actual\n\n{what}.\n\n\
         <details><summary>stderr (last lines)</summary>\n\n```\n{stderr}\n```\n</details>\n\n\
         <details><summary>stdout (last lines)</summary>\n\n```\n{stdout}\n```\n</details>\n\n\
         <!-- Draft from STORYCRAFT. Attach a minimal input file and recheck the latest release before filing. -->\n",
        ver = s(&f, "app_version"),
        os = s(&f, "os"),
        arch = s(&f, "arch"),
        stderr = s(&f, "stderr_tail"),
        stdout = s(&f, "stdout_tail"),
    );
    let (title, body) = (scrub(&title), scrub(&body));
    let path = job_dir.join("bug-draft.md");
    std::fs::write(&path, format!("# {title}\n\n{body}")).map_err(|e| Error::io(&path, e))?;
    Ok(Draft { repo: format!("storytold/{app}"), title, body, path })
}

/// Quote one argument for a POSIX shell.
pub fn shell_quote(arg: &str) -> String {
    if !arg.is_empty() && arg.bytes().all(|b| b.is_ascii_alphanumeric() || b"-_./=:@+,".contains(&b)) {
        arg.to_string()
    } else {
        format!("'{}'", arg.replace('\'', "'\\''"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn draft_quotes_the_command_and_names_the_repo() -> Result<()> {
        let dir = tempfile::tempdir().map_err(|e| Error::Other(e.to_string()))?;
        let failure = serde_json::json!({
            "app": "photocraft", "app_version": "0.23.1", "step": "compose",
            "command": ["photocraft-cli", "run", "in/image.png", "--params", "{\"a\": 1}"],
            "exit_code": 1, "timed_out": false, "stderr_tail": "boom", "stdout_tail": "", "os": "macos", "arch": "aarch64"
        });
        std::fs::write(dir.path().join("failure.json"), failure.to_string()).map_err(|e| Error::io(dir.path(), e))?;
        let d = draft(dir.path())?;
        assert_eq!(d.repo, "storytold/photocraft");
        assert!(d.body.contains("'{\"a\": 1}'"), "{}", d.body);
        assert!(d.title.contains("exited with 1"));
        assert!(d.path.is_file());
        Ok(())
    }
}
