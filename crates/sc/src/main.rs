//! `sc`: the STORYCRAFT command line.
//!
//! Exit codes: 0 ok, 1 operation failed, 2 usage error, 3 a verifier failed.

use std::collections::BTreeMap;
use std::io::{BufRead, Write};
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;

use clap::{Parser, Subcommand};
use storycraft_core::doctor::{self, Status};
use storycraft_core::{Error, apps, bug, engine, flags, paths, recipe, verify};

#[derive(Parser)]
#[command(name = "sc", version, about = "Drive the ArtCraft Crafting Apps: setup checks, recipes, verification, bug drafts")]
struct Cli {
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Check this Mac is ready: AI-MCP flag, launcher agents switch, app CLIs, MCP clients
    Doctor {
        /// Offer each available fix and apply the ones you agree to
        #[arg(long)]
        fix: bool,
        /// With --fix: apply every fix without asking
        #[arg(long, requires = "fix")]
        yes: bool,
        #[arg(long)]
        json: bool,
    },
    /// Read or change Craft suite flags (ArtCraft/flags.toml)
    Flags {
        #[command(subcommand)]
        action: FlagsCmd,
    },
    /// List the Craft apps STORYCRAFT drives and where their CLIs are
    Apps {
        #[arg(long)]
        json: bool,
    },
    /// Print an app's command catalog (`<app>-cli commands`)
    Commands {
        app: String,
        /// Only lines containing this text
        #[arg(long)]
        search: Option<String>,
    },
    /// List the recipes in the recipes folder
    Recipes,
    /// Run a recipe by id or path
    Run {
        recipe: String,
        /// Recipe inputs, repeatable: --set title="Big News" --set image=photo.jpg
        #[arg(long = "set", value_name = "NAME=VALUE")]
        set: Vec<String>,
        /// Job folder to use instead of a new one under ~/StoryCraft/jobs
        #[arg(long)]
        job_dir: Option<PathBuf>,
        /// Print the resolved commands; run nothing
        #[arg(long)]
        dry_run: bool,
        /// Use this CLI for an app instead of discovery, repeatable: --app photocraft=/path/photocraft-cli
        #[arg(long = "app", value_name = "APP=PATH")]
        app: Vec<String>,
        #[arg(long)]
        json: bool,
    },
    /// Check a file against expectations, as recipes do
    Verify {
        file: PathBuf,
        #[arg(long)]
        width: Option<u32>,
        #[arg(long)]
        height: Option<u32>,
        #[arg(long)]
        max_bytes: Option<u64>,
        /// png, jpeg, svg, ico, icns or psd
        #[arg(long)]
        format: Option<String>,
    },
    /// Bug reports (drafts only; filing needs your approval per issue)
    Bug {
        #[command(subcommand)]
        action: BugCmd,
    },
}

#[derive(Subcommand)]
enum FlagsCmd {
    /// Show a flag (default AI-MCP): the file, CRAFT_FLAGS, and the result
    Status {
        #[arg(default_value = flags::AI_MCP)]
        flag: String,
    },
    /// Turn a flag on in the flags file (asks first unless --yes)
    Enable {
        #[arg(default_value = flags::AI_MCP)]
        flag: String,
        #[arg(long)]
        yes: bool,
    },
    /// Turn a flag off in the flags file (asks first unless --yes)
    Disable {
        #[arg(default_value = flags::AI_MCP)]
        flag: String,
        #[arg(long)]
        yes: bool,
    },
}

#[derive(Subcommand)]
enum BugCmd {
    /// Write bug-draft.md from a failed job's failure.json
    Draft { job_dir: PathBuf },
}

fn confirm(question: &str) -> bool {
    print!("{question} [y/N] ");
    let _ = std::io::stdout().flush();
    let mut line = String::new();
    if std::io::stdin().lock().read_line(&mut line).is_err() {
        return false;
    }
    matches!(line.trim().to_ascii_lowercase().as_str(), "y" | "yes")
}

fn print_json(v: &impl serde::Serialize) -> anyhow::Result<()> {
    println!("{}", serde_json::to_string_pretty(v)?);
    Ok(())
}

fn doctor_cmd(fix: bool, yes: bool, json: bool) -> anyhow::Result<u8> {
    let report = doctor::run(&doctor::Options::default());
    if json {
        print_json(&report)?;
    } else {
        for c in &report.checks {
            let mark = match c.status {
                Status::Ok => "ok  ",
                Status::Warn => "warn",
                Status::Fail => "FAIL",
            };
            println!("{mark}  {:<32} {}", c.id, c.detail);
            if let Some(h) = &c.hint {
                println!("      {:<32} -> {h}", "");
            }
        }
    }
    if fix {
        for f in report.fixes() {
            if yes || confirm(&format!("Fix: {}?", f.describe())) {
                f.apply()?;
                println!("done: {}", f.describe());
            }
        }
        if !report.fixes().is_empty() {
            println!("Restart the ArtCraft Launcher and any open Craft apps, then run `sc doctor` again.");
        }
    } else if !json && !report.fixes().is_empty() {
        println!("\n{} fix(es) available: run `sc doctor --fix`", report.fixes().len());
    }
    Ok(if report.worst() == Status::Fail { 1 } else { 0 })
}

fn flags_cmd(action: FlagsCmd) -> anyhow::Result<u8> {
    let path = paths::flags_file().ok_or_else(|| anyhow::anyhow!("no per-user configuration folder"))?;
    let env = std::env::var("CRAFT_FLAGS").ok();
    let (flag, on, yes) = match action {
        FlagsCmd::Status { flag } => {
            let s = flags::state(&path, env.as_deref(), &flag)?;
            println!(
                "{}: {} (file {}: {:?}; CRAFT_FLAGS: {:?})",
                s.id,
                if s.enabled { "on" } else { "off" },
                path.display(),
                s.file,
                s.env
            );
            return Ok(0);
        }
        FlagsCmd::Enable { flag, yes } => (flag, true, yes),
        FlagsCmd::Disable { flag, yes } => (flag, false, yes),
    };
    let verb = if on { "on" } else { "off" };
    if !yes && !confirm(&format!("Turn {flag} {verb} in {}?", path.display())) {
        println!("unchanged");
        return Ok(0);
    }
    flags::set(&path, &flag, on)?;
    println!("{flag} is {verb} in {}", path.display());
    if flags::env_value(env.as_deref().unwrap_or(""), &flag).is_some_and(|v| v != on) {
        println!("note: CRAFT_FLAGS in this shell overrides the file");
    }
    println!("Restart the ArtCraft Launcher and any open Craft apps to pick it up.");
    Ok(0)
}

fn apps_cmd(json: bool) -> anyhow::Result<u8> {
    let rows: Vec<serde_json::Value> = apps::PRIORITY
        .iter()
        .map(|s| {
            let found = apps::find_cli(s);
            let version = found.as_ref().and_then(|f| apps::version(&f.path, Duration::from_secs(20)).ok());
            serde_json::json!({ "id": s.id, "name": s.name, "cli": found.as_ref().map(|f| &f.path), "source": found.as_ref().map(|f| &f.source), "version": version })
        })
        .collect();
    if json {
        print_json(&rows)?;
    } else {
        for r in &rows {
            println!(
                "{:<12} {:<8} {}",
                r["id"].as_str().unwrap_or(""),
                r["version"].as_str().unwrap_or("-"),
                r["cli"].as_str().unwrap_or("not found")
            );
        }
    }
    Ok(0)
}

fn commands_cmd(app: &str, search: Option<&str>) -> anyhow::Result<u8> {
    let spec = apps::spec(app).ok_or_else(|| anyhow::anyhow!("`{app}` is not a Craft app id"))?;
    let cli = apps::require_cli(&spec)?;
    let catalog = apps::commands(&cli.path, Duration::from_secs(30))?;
    for line in catalog.lines() {
        if search.is_none_or(|s| line.to_ascii_lowercase().contains(&s.to_ascii_lowercase())) {
            println!("{line}");
        }
    }
    Ok(0)
}

fn recipes_cmd() -> anyhow::Result<u8> {
    let dir = paths::recipes_dir().ok_or_else(|| anyhow::anyhow!("no recipes folder; set SC_RECIPES_DIR"))?;
    let mut code = 0;
    for r in recipe::list(&dir)? {
        match r {
            Ok(r) => println!("{:<20} {}", r.id, r.summary),
            Err(e) => {
                eprintln!("{e}");
                code = 1;
            }
        }
    }
    Ok(code)
}

fn parse_sets(sets: &[String]) -> anyhow::Result<BTreeMap<String, String>> {
    sets.iter()
        .map(|s| {
            s.split_once('=')
                .map(|(k, v)| (k.trim().to_string(), v.to_string()))
                .ok_or_else(|| anyhow::anyhow!("`{s}`: expected NAME=VALUE"))
        })
        .collect()
}

fn run_cmd(name: &str, sets: &[String], apps: &[String], job_dir: Option<PathBuf>, dry_run: bool, json: bool) -> anyhow::Result<u8> {
    let r = recipe::Recipe::load(&recipe::resolve(name)?)?;
    let app_overrides = parse_sets(apps)?.into_iter().map(|(k, v)| (k, PathBuf::from(v))).collect();
    let opts = engine::RunOptions { inputs: parse_sets(sets)?, job_dir, dry_run, app_overrides };
    match engine::run(&r, &opts) {
        Ok(report) => {
            if json {
                print_json(&report)?;
            } else {
                for s in &report.steps {
                    let state = if s.skipped {
                        "skip"
                    } else if dry_run {
                        "plan"
                    } else {
                        "done"
                    };
                    println!("{state}  {}", s.id);
                    if let Some(cmd) = &s.command {
                        println!("      {}", cmd.iter().map(|a| bug::shell_quote(a)).collect::<Vec<_>>().join(" "));
                    }
                }
                if !dry_run {
                    println!("job: {}", report.job_dir.display());
                }
            }
            Ok(0)
        }
        Err(e) => {
            eprintln!("error: {e}");
            if matches!(e, Error::Step { .. }) {
                eprintln!(
                    "a failure report was written; draft a bug with `sc bug draft <job dir>` (nothing is filed without your approval)"
                );
            }
            Ok(e.exit_code())
        }
    }
}

fn verify_cmd(
    file: PathBuf,
    width: Option<u32>,
    height: Option<u32>,
    max_bytes: Option<u64>,
    format: Option<String>,
) -> anyhow::Result<u8> {
    let format = format.map(|f| serde_json::from_value::<verify::Format>(serde_json::Value::String(f.to_ascii_lowercase()))).transpose()?;
    let rule = verify::Rule { file: String::new(), format, width, height, max_bytes, min_bytes: None, alpha: None, min_images: None };
    let outcome = verify::check(&file, &rule);
    if outcome.ok {
        println!("ok  {}", file.display());
        Ok(0)
    } else {
        for p in &outcome.problems {
            println!("FAIL {}: {p}", file.display());
        }
        Ok(3)
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let result = match cli.command {
        Cmd::Doctor { fix, yes, json } => doctor_cmd(fix, yes, json),
        Cmd::Flags { action } => flags_cmd(action),
        Cmd::Apps { json } => apps_cmd(json),
        Cmd::Commands { app, search } => commands_cmd(&app, search.as_deref()),
        Cmd::Recipes => recipes_cmd(),
        Cmd::Run { recipe, set, job_dir, dry_run, app, json } => run_cmd(&recipe, &set, &app, job_dir, dry_run, json),
        Cmd::Verify { file, width, height, max_bytes, format } => verify_cmd(file, width, height, max_bytes, format),
        Cmd::Bug { action: BugCmd::Draft { job_dir } } => bug::draft(&job_dir)
            .map(|d| {
                println!("draft: {}\nrepo:  {}\ntitle: {}", d.path.display(), d.repo, d.title);
                println!("Review it; file only after you approve this specific issue.");
                0
            })
            .map_err(anyhow::Error::from),
    };
    match result {
        Ok(code) => ExitCode::from(code),
        Err(e) => {
            eprintln!("error: {e:#}");
            ExitCode::from(1)
        }
    }
}
