//! `sc doctor`: is this Mac ready for agents to drive the Craft apps?
//!
//! Every check reads only; fixes are separate and each one is applied only after the user agrees
//! (or passes `--yes`). The only things a fix may write are the AI-MCP flag in `flags.toml` and a
//! `STORYCRAFT` entry in an MCP client's settings.

use std::path::PathBuf;
use std::time::Duration;

use crate::apps::{self, PRIORITY};
use crate::clients::{self, Client};
use crate::{Result, flags, paths};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Ok,
    Warn,
    Fail,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Fix {
    /// Set `AI-MCP = true` in `flags.toml`.
    EnableAiMcp { flags_file: PathBuf },
    /// Add the `STORYCRAFT` server to a client's MCP settings.
    AddStorycraftServer { client: Client, config: PathBuf, command: PathBuf },
}

impl Fix {
    pub fn describe(&self) -> String {
        match self {
            Fix::EnableAiMcp { flags_file } => format!("set `AI-MCP = true` under [flags] in {}", flags_file.display()),
            Fix::AddStorycraftServer { client, config, command } => {
                format!("add the STORYCRAFT MCP server ({}) to {} ({})", command.display(), client.name(), config.display())
            }
        }
    }

    pub fn apply(&self) -> Result<()> {
        match self {
            Fix::EnableAiMcp { flags_file } => flags::set(flags_file, flags::AI_MCP, true),
            Fix::AddStorycraftServer { config, command, .. } => clients::put_server(config, clients::STORYCRAFT_KEY, command, &[]),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Check {
    pub id: String,
    pub status: Status,
    pub detail: String,
    /// What a person can do about a warning or failure.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fix: Option<Fix>,
}

impl Check {
    fn new(id: impl Into<String>, status: Status, detail: impl Into<String>) -> Self {
        Self { id: id.into(), status, detail: detail.into(), hint: None, fix: None }
    }
    fn hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }
    fn fix(mut self, fix: Fix) -> Self {
        self.fix = Some(fix);
        self
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Report {
    pub checks: Vec<Check>,
}

impl Report {
    pub fn worst(&self) -> Status {
        self.checks.iter().map(|c| c.status).max().unwrap_or(Status::Ok)
    }
    pub fn fixes(&self) -> Vec<&Fix> {
        self.checks.iter().filter_map(|c| c.fix.as_ref()).collect()
    }
}

pub struct Options {
    pub timeout: Duration,
    /// The `storycraft-mcp` binary to offer for client settings (usually beside `sc`).
    pub storycraft_mcp: Option<PathBuf>,
}

impl Default for Options {
    fn default() -> Self {
        let beside = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.join(format!("storycraft-mcp{}", std::env::consts::EXE_SUFFIX))))
            .filter(|p| p.is_file());
        Self { timeout: Duration::from_secs(20), storycraft_mcp: beside }
    }
}

fn platform() -> Check {
    if cfg!(target_os = "macos") {
        Check::new("platform", Status::Ok, "macOS")
    } else {
        Check::new("platform", Status::Warn, format!("{}: app checks assume macOS install paths", std::env::consts::OS))
            .hint("STORYCRAFT targets the Mac; on other systems set SC_<APP>_CLI for each app")
    }
}

fn ai_mcp_flag() -> Check {
    let Some(path) = paths::flags_file() else {
        return Check::new("flag.ai-mcp", Status::Fail, "no per-user configuration folder on this system");
    };
    let env = std::env::var("CRAFT_FLAGS").ok();
    match flags::state(&path, env.as_deref(), flags::AI_MCP) {
        Ok(s) if s.enabled && s.file == Some(true) => Check::new("flag.ai-mcp", Status::Ok, format!("AI-MCP is on in {}", path.display())),
        Ok(s) if s.enabled => Check::new("flag.ai-mcp", Status::Warn, "AI-MCP is on for this run only (CRAFT_FLAGS)")
            .hint("apps started from the Dock do not see CRAFT_FLAGS; turn it on in the file")
            .fix(Fix::EnableAiMcp { flags_file: path }),
        Ok(s) => {
            let why = if s.env == Some(false) { "CRAFT_FLAGS turns it off for this run" } else { "not set" };
            Check::new("flag.ai-mcp", Status::Fail, format!("AI-MCP is off ({why}); {}", path.display()))
                .hint("run `sc flags enable AI-MCP`, then restart the launcher and the apps")
                .fix(Fix::EnableAiMcp { flags_file: path })
        }
        Err(e) => Check::new("flag.ai-mcp", Status::Fail, e.to_string())
            .hint("fix or move the flags file by hand; sc will not overwrite a file it cannot parse"),
    }
}

fn launcher() -> Check {
    let candidates = ["/Applications/ArtCraft Launcher.app", "/Applications/ArtCraft.app"];
    match candidates.iter().find(|p| std::path::Path::new(p).is_dir()) {
        Some(p) => Check::new("launcher", Status::Ok, *p),
        None => Check::new("launcher", Status::Warn, "ArtCraft Launcher not found in /Applications")
            .hint("install it from https://getartcraft.com/apps; it installs the apps and the agent gateway"),
    }
}

fn agents_switch() -> Check {
    let Some(dir) = paths::mcp_dir() else {
        return Check::new("launcher.agents", Status::Fail, "no per-user configuration folder on this system");
    };
    match craft_ai_agents::load(&dir) {
        Ok(Some((apps, problems))) => {
            let ids: Vec<&str> = apps.iter().map(|a| a.id.as_str()).collect();
            let status = if problems.is_empty() { Status::Ok } else { Status::Warn };
            let mut c = Check::new("launcher.agents", status, format!("on; apps.json lists {}", ids.join(", ")));
            if !problems.is_empty() {
                c = c.hint(problems.join("; "));
            }
            c
        }
        Ok(None) => Check::new("launcher.agents", Status::Fail, format!("off (no {})", dir.join(craft_ai_agents::APPS_FILE).display()))
            .hint("in the ArtCraft Launcher, open Settings and turn on \"Let AI agents use these apps\" (needs the AI-MCP flag)"),
        Err(e) => Check::new("launcher.agents", Status::Fail, e).hint("turn the launcher switch off and on again to rewrite apps.json"),
    }
}

fn gateway() -> Check {
    match paths::gateway_copy() {
        Some(p) if p.is_file() => Check::new("gateway", Status::Ok, p.display().to_string()),
        Some(p) => Check::new("gateway", Status::Warn, format!("{} missing", p.display()))
            .hint("the launcher copies craft-ai-mcp there when the agents switch is turned on"),
        None => Check::new("gateway", Status::Warn, "no per-user configuration folder"),
    }
}

fn app(spec: &apps::AppSpec, timeout: Duration) -> Check {
    let id = format!("app.{}", spec.id);
    let Some(found) = apps::find_cli(spec) else {
        return Check::new(id, Status::Fail, format!("{} not found", spec.cli_name())).hint(format!(
            "install {} with the ArtCraft Launcher, or set {}",
            spec.name,
            spec.env_var()
        ));
    };
    let listed = apps::launcher_entry(spec.id).is_some();
    match apps::version(&found.path, timeout) {
        Ok(v) => {
            let mut c = Check::new(id, Status::Ok, format!("{} {v} ({:?}: {})", spec.cli_name(), found.source, found.path.display()));
            if !listed {
                c.status = Status::Warn;
                c = c.hint(format!("{} is not in the launcher's apps.json, so the ArtCraft gateway will not offer it", spec.name));
            }
            c
        }
        Err(e) => Check::new(id, Status::Fail, e.to_string()).hint("the CLI did not answer --version; run it by hand to see why"),
    }
}

fn client(client: Client, storycraft_mcp: Option<&PathBuf>) -> Vec<Check> {
    let Some(path) = client.config_path() else { return vec![] };
    let key = |k: &str| format!("client.{}.{k}", client.name().to_ascii_lowercase().replace(' ', "-"));
    let mut out = Vec::new();
    match clients::server(&path, clients::ARTCRAFT_KEY) {
        Ok(Some(_)) => out.push(Check::new(key("artcraft"), Status::Ok, format!("ArtCraft gateway listed in {}", path.display()))),
        Ok(None) => out.push(
            Check::new(key("artcraft"), Status::Warn, format!("no ArtCraft server in {}", path.display()))
                .hint(format!("in the launcher's agents panel, add {}", client.name())),
        ),
        Err(e) => out.push(Check::new(key("artcraft"), Status::Warn, e.to_string())),
    }
    match clients::server(&path, clients::STORYCRAFT_KEY) {
        Ok(Some(_)) => out.push(Check::new(key("storycraft"), Status::Ok, "STORYCRAFT server listed")),
        Ok(None) => {
            let mut c = Check::new(key("storycraft"), Status::Warn, "no STORYCRAFT server (workflow tools)");
            match storycraft_mcp {
                Some(cmd) => c = c.fix(Fix::AddStorycraftServer { client, config: path, command: cmd.clone() }),
                None => c = c.hint("install storycraft-mcp beside sc (`cargo install --path crates/mcp`), then rerun with --fix"),
            }
            out.push(c)
        }
        Err(e) => out.push(Check::new(key("storycraft"), Status::Warn, e.to_string())),
    }
    out
}

fn gh(timeout: Duration) -> Check {
    match apps::exec(std::path::Path::new("gh"), &["auth".into(), "status".into()], None, timeout) {
        Ok(o) if o.ok() => Check::new("gh", Status::Ok, "GitHub CLI signed in (needed only to file approved bugs)"),
        Ok(_) => Check::new("gh", Status::Warn, "GitHub CLI not signed in").hint("run `gh auth login` before filing approved bug reports"),
        Err(_) => {
            Check::new("gh", Status::Warn, "GitHub CLI not installed").hint("`brew install gh`; only needed to file approved bug reports")
        }
    }
}

pub fn run(opts: &Options) -> Report {
    let mut checks = vec![platform(), ai_mcp_flag(), launcher(), agents_switch(), gateway()];
    checks.extend(PRIORITY.iter().map(|s| app(s, opts.timeout)));
    for c in Client::ALL {
        checks.extend(client(c, opts.storycraft_mcp.as_ref()));
    }
    checks.push(gh(opts.timeout));
    Report { checks }
}
