//! Finding each Craft app's command-line tool and asking it what it is.
//!
//! Lookup order for an app's CLI (first hit wins):
//! 1. `SC_<APP>_CLI` (e.g. `SC_PHOTOCRAFT_CLI=/path/to/photocraft-cli`);
//! 2. the launcher's `ArtCraft/mcp/apps.json` entry (written when "Let AI agents use these apps" is on);
//! 3. beside the main executable in `/Applications/<Name>.app/Contents/MacOS/` or `~/Applications/…`
//!    (where the launcher's detection looks for `<app>-cli`);
//! 4. `<app>-cli` on `PATH` (a release zip unpacked by hand).

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use crate::{Error, Result, paths};

/// The apps STORYCRAFT drives today, in priority order (project decision D3).
pub const PRIORITY: &[AppSpec] = &[
    AppSpec { id: "photocraft", name: "PhotoCraft" },
    AppSpec { id: "vectorcraft", name: "VectorCraft" },
    AppSpec { id: "filmcraft", name: "FilmCraft" },
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AppSpec {
    pub id: &'static str,
    pub name: &'static str,
}

impl AppSpec {
    pub fn cli_name(&self) -> String {
        format!("{}-cli{}", self.id, std::env::consts::EXE_SUFFIX)
    }

    pub fn env_var(&self) -> String {
        format!("SC_{}_CLI", self.id.to_ascii_uppercase())
    }
}

/// Any suite app id, priority or not; names come from the launcher's own list.
pub fn spec(id: &str) -> Option<AppSpec> {
    PRIORITY.iter().copied().find(|a| a.id == id).or_else(|| {
        let name = craft_ai_agents::known_name(id)?;
        let id = craft_ai_agents::KNOWN_APPS.iter().find(|(i, _)| *i == id)?.0;
        Some(AppSpec { id, name })
    })
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub enum Source {
    Override,
    Env,
    LauncherList,
    AppBundle,
    Path,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct FoundCli {
    pub app: String,
    pub path: PathBuf,
    pub source: Source,
}

fn bundle_candidates(spec: &AppSpec) -> Vec<PathBuf> {
    let mut roots = vec![PathBuf::from("/Applications")];
    if let Some(home) = paths::home() {
        roots.push(home.join("Applications"));
    }
    roots.into_iter().map(|r| r.join(format!("{}.app", spec.name)).join("Contents").join("MacOS").join(spec.cli_name())).collect()
}

fn on_path(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path).map(|d| d.join(name)).find(|p| p.is_file())
}

pub fn launcher_entry(app_id: &str) -> Option<craft_ai_agents::AppEntry> {
    let dir = paths::mcp_dir()?;
    let (apps, _) = craft_ai_agents::load(&dir).ok()??;
    apps.into_iter().find(|a| a.id == app_id)
}

pub fn find_cli(spec: &AppSpec) -> Option<FoundCli> {
    let found = |path: PathBuf, source| Some(FoundCli { app: spec.id.to_string(), path, source });
    if let Some(p) = std::env::var_os(spec.env_var()).filter(|v| !v.is_empty()) {
        return found(PathBuf::from(p), Source::Env);
    }
    if let Some(entry) = launcher_entry(spec.id).filter(|e| e.command.is_file()) {
        return found(entry.command, Source::LauncherList);
    }
    if let Some(p) = bundle_candidates(spec).into_iter().find(|p| p.is_file()) {
        return found(p, Source::AppBundle);
    }
    on_path(&spec.cli_name()).and_then(|p| found(p, Source::Path))
}

pub fn require_cli(spec: &AppSpec) -> Result<FoundCli> {
    find_cli(spec).ok_or_else(|| Error::AppMissing {
        app: spec.id.to_string(),
        hint: format!("install it with the ArtCraft Launcher, or set {} to its {}", spec.env_var(), spec.cli_name()),
    })
}

/// What a command printed, and how it ended.
#[derive(Debug, Clone)]
pub struct Output {
    pub code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub timed_out: bool,
    pub elapsed: Duration,
}

impl Output {
    pub fn ok(&self) -> bool {
        !self.timed_out && self.code == Some(0)
    }
}

/// Run `program args…` with no stdin, capturing output, killing it after `timeout`.
pub fn exec(program: &Path, args: &[String], cwd: Option<&Path>, timeout: Duration) -> Result<Output> {
    let started = Instant::now();
    let mut cmd = Command::new(program);
    cmd.args(args).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());
    if let Some(dir) = cwd {
        cmd.current_dir(dir);
    }
    let mut child = cmd.spawn().map_err(|e| Error::io(program, e))?;
    let mut out_pipe = child.stdout.take();
    let mut err_pipe = child.stderr.take();
    let out_thread = std::thread::spawn(move || {
        let mut s = String::new();
        if let Some(p) = out_pipe.as_mut() {
            let _ = p.read_to_string(&mut s);
        }
        s
    });
    let err_thread = std::thread::spawn(move || {
        let mut s = String::new();
        if let Some(p) = err_pipe.as_mut() {
            let _ = p.read_to_string(&mut s);
        }
        s
    });
    let mut timed_out = false;
    let status = loop {
        match child.try_wait().map_err(|e| Error::io(program, e))? {
            Some(status) => break Some(status),
            None if started.elapsed() >= timeout => {
                timed_out = true;
                let _ = child.kill();
                break child.wait().ok();
            }
            None => std::thread::sleep(Duration::from_millis(25)),
        }
    };
    Ok(Output {
        code: status.and_then(|s| s.code()),
        stdout: out_thread.join().unwrap_or_default(),
        stderr: err_thread.join().unwrap_or_default(),
        timed_out,
        elapsed: started.elapsed(),
    })
}

/// The first dotted number in `text` (`photocraft-cli 0.23.1` -> `0.23.1`).
pub fn parse_version(text: &str) -> Option<String> {
    text.split(|c: char| c.is_whitespace() || c == 'v' || c == '(' || c == ')')
        .find(|tok| tok.contains('.') && tok.split('.').all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit())))
        .map(str::to_string)
}

fn version_parts(v: &str) -> Vec<u64> {
    v.split('.').map(|p| p.parse().unwrap_or(0)).collect()
}

/// `found >= min`, comparing dotted numbers part by part.
pub fn version_at_least(found: &str, min: &str) -> bool {
    let (mut a, mut b) = (version_parts(found), version_parts(min));
    let n = a.len().max(b.len());
    a.resize(n, 0);
    b.resize(n, 0);
    a >= b
}

pub fn version(cli: &Path, timeout: Duration) -> Result<String> {
    let out = exec(cli, &["--version".to_string()], None, timeout)?;
    parse_version(&out.stdout)
        .or_else(|| parse_version(&out.stderr))
        .ok_or_else(|| Error::Other(format!("{} --version printed no version: {}", cli.display(), out.stdout.trim())))
}

/// The app's command catalog, as `<app>-cli commands` prints it (text; ids are looked up in it).
pub fn commands(cli: &Path, timeout: Duration) -> Result<String> {
    let out = exec(cli, &["commands".to_string()], None, timeout)?;
    if !out.ok() {
        return Err(Error::Other(format!("{} commands failed: {}", cli.display(), out.stderr.trim())));
    }
    Ok(out.stdout)
}

/// Whether a command catalog lists `id` as a whole token.
pub fn catalog_has(catalog: &str, id: &str) -> bool {
    catalog.split(|c: char| !(c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-')).any(|tok| tok == id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions_are_read_and_compared() {
        assert_eq!(parse_version("photocraft-cli 0.23.1").as_deref(), Some("0.23.1"));
        assert_eq!(parse_version("VectorCraft v0.24.0 (abc)").as_deref(), Some("0.24.0"));
        assert!(version_at_least("0.23.1", "0.23"));
        assert!(!version_at_least("0.18.9", "0.19.0"));
    }

    #[test]
    fn catalog_lookup_matches_whole_ids() {
        let cat = "image.imageSize  Image Size…\nimage.crop  Crop\n";
        assert!(catalog_has(cat, "image.crop"));
        assert!(!catalog_has(cat, "image.cro"));
    }

    #[test]
    fn non_priority_apps_resolve_from_the_launcher_list() {
        assert_eq!(spec("photocraft").map(|s| s.name), Some("PhotoCraft"));
        assert!(spec("not-an-app").is_none());
    }
}
