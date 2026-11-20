//! Running a recipe: inputs -> job folder -> preflight -> steps -> verify -> `job.json`.
//!
//! Each run gets its own folder (`<jobs>/<recipe>-<time>-<pid>/` with `in/` and `out/`); inputs are
//! copied into `in/` and the apps only read `in/` and write `out/`. A failed step writes
//! `failure.json` there, which `sc bug draft` turns into a report for approval.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::Serialize;
use serde_json::json;

use crate::apps::{self, FoundCli};
use crate::recipe::{Builtin, InputKind, Recipe, parse_hex_color, render, when_holds};
use crate::{Error, Result, pack, paths, verify};

#[derive(Debug, Default, Clone)]
pub struct RunOptions {
    pub inputs: BTreeMap<String, String>,
    /// Use this folder instead of a new one under the jobs folder.
    pub job_dir: Option<PathBuf>,
    /// Resolve and print every command; run nothing, write nothing.
    pub dry_run: bool,
    /// Use these CLI binaries instead of discovery (`photocraft` -> `/path/photocraft-cli`).
    pub app_overrides: BTreeMap<String, PathBuf>,
}

#[derive(Debug, Clone, Serialize)]
pub struct StepRecord {
    pub id: String,
    pub skipped: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    pub millis: u128,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub outputs: Vec<PathBuf>,
}

#[derive(Debug, Clone, Serialize)]
pub struct JobReport {
    pub recipe: String,
    pub recipe_version: u32,
    pub job_dir: PathBuf,
    pub dry_run: bool,
    pub apps: BTreeMap<String, AppUse>,
    pub inputs: BTreeMap<String, String>,
    pub steps: Vec<StepRecord>,
    pub verify: Vec<verify::Outcome>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AppUse {
    pub cli: PathBuf,
    pub version: String,
}

fn now_secs() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

fn new_job_dir(recipe: &str) -> Result<PathBuf> {
    let root = paths::jobs_dir().ok_or_else(|| Error::Other("no home folder for jobs; set SC_JOBS_DIR".into()))?;
    Ok(root.join(format!("{recipe}-{}-{}", now_secs(), std::process::id())))
}

fn mkdir(p: &Path) -> Result<()> {
    std::fs::create_dir_all(p).map_err(|e| Error::io(p, e))
}

/// Check given inputs against the recipe, fill defaults, and copy files into `in/`.
fn prepare_inputs(recipe: &Recipe, given: &BTreeMap<String, String>, in_dir: &Path, dry_run: bool) -> Result<BTreeMap<String, String>> {
    for name in given.keys() {
        if !recipe.inputs.contains_key(name) {
            let known: Vec<&str> = recipe.inputs.keys().map(String::as_str).collect();
            return Err(Error::Recipe(format!("`{}` has no input `{name}` (inputs: {})", recipe.id, known.join(", "))));
        }
    }
    let mut out = BTreeMap::new();
    for (name, spec) in &recipe.inputs {
        let value = given.get(name).cloned().or_else(|| spec.default.clone()).unwrap_or_default();
        if value.trim().is_empty() {
            if spec.required {
                return Err(Error::Recipe(format!(
                    "input `{name}` is required{}",
                    spec.help.as_deref().map(|h| format!(": {h}")).unwrap_or_default()
                )));
            }
            out.insert(name.clone(), String::new());
            continue;
        }
        let value = match spec.kind {
            InputKind::Text => value,
            InputKind::Number => {
                value.trim().parse::<f64>().map_err(|_| Error::Recipe(format!("input `{name}`: `{value}` is not a number")))?;
                value.trim().to_string()
            }
            InputKind::Color => {
                parse_hex_color(&value).ok_or_else(|| Error::Recipe(format!("input `{name}`: `{value}` is not a #rrggbb colour")))?;
                value.trim().to_string()
            }
            InputKind::File => {
                let src = PathBuf::from(&value);
                if !src.is_file() {
                    return Err(Error::Recipe(format!("input `{name}`: no file at {}", src.display())));
                }
                let ext = src.extension().map(|e| e.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
                if !spec.extensions.is_empty() && !spec.extensions.contains(&ext) {
                    return Err(Error::Recipe(format!("input `{name}`: .{ext} is not one of {}", spec.extensions.join(", "))));
                }
                if dry_run {
                    src.display().to_string()
                } else {
                    let dst = in_dir.join(format!("{name}.{ext}"));
                    std::fs::copy(&src, &dst).map_err(|e| Error::io(&dst, e))?;
                    dst.display().to_string()
                }
            }
        };
        out.insert(name.clone(), value);
    }
    Ok(out)
}

/// Find every app the recipe needs, check its version and command ids.
/// `only`: check just these apps (the ones whose steps will run); `None` checks every app the recipe names.
pub fn preflight(
    recipe: &Recipe,
    timeout: Duration,
    overrides: &BTreeMap<String, PathBuf>,
    only: Option<&std::collections::BTreeSet<String>>,
) -> Result<BTreeMap<String, (FoundCli, String)>> {
    let mut found = BTreeMap::new();
    for (app_id, req) in recipe.requires.iter().filter(|(id, _)| only.is_none_or(|o| o.contains(*id))) {
        let spec = apps::spec(app_id).ok_or_else(|| Error::Recipe(format!("`{app_id}` is not a Craft app id")))?;
        let cli = match overrides.get(app_id) {
            Some(path) => FoundCli { app: app_id.clone(), path: path.clone(), source: apps::Source::Override },
            None => apps::require_cli(&spec)?,
        };
        let version = apps::version(&cli.path, timeout)?;
        if let Some(min) = &req.min_version
            && !apps::version_at_least(&version, min)
        {
            return Err(Error::AppTooOld { app: app_id.clone(), found: version, min: min.clone() });
        }
        if !req.commands.is_empty() {
            let catalog = apps::commands(&cli.path, timeout)?;
            if let Some(missing) = req.commands.iter().find(|c| !apps::catalog_has(&catalog, c)) {
                return Err(Error::CommandMissing { app: app_id.clone(), version, command: missing.clone() });
            }
        }
        found.insert(app_id.clone(), (cli, version));
    }
    Ok(found)
}

fn tail(s: &str, lines: usize) -> String {
    let all: Vec<&str> = s.lines().collect();
    all[all.len().saturating_sub(lines)..].join("\n")
}

fn write_json(path: &Path, value: &impl Serialize) -> Result<()> {
    let text = serde_json::to_string_pretty(value)?;
    std::fs::write(path, text + "\n").map_err(|e| Error::io(path, e))
}

fn cover_fit(image: &Path, width: u32, height: u32) -> Result<[(String, String); 4]> {
    let (w, h) = image::image_dimensions(image)
        .map_err(|e| Error::Other(format!("{}: cannot read its size ({e}); give a PNG or JPEG", image.display())))?;
    let scale = (f64::from(width) / f64::from(w)).max(f64::from(height) / f64::from(h));
    let nw = (f64::from(w) * scale).ceil().max(f64::from(width)) as u32;
    let nh = (f64::from(h) * scale).ceil().max(f64::from(height)) as u32;
    Ok([
        ("width".into(), nw.to_string()),
        ("height".into(), nh.to_string()),
        ("x".into(), ((nw - width) / 2).to_string()),
        ("y".into(), ((nh - height) / 2).to_string()),
    ])
}

/// Initials: words split on spaces, `-`, `_` and lower-to-upper case changes.
pub fn monogram(text: &str, max: usize) -> String {
    let mut words: Vec<String> = Vec::new();
    let mut cur = String::new();
    let mut prev_lower = false;
    for ch in text.chars() {
        if ch.is_whitespace() || ch == '-' || ch == '_' {
            if !cur.is_empty() {
                words.push(std::mem::take(&mut cur));
            }
            prev_lower = false;
            continue;
        }
        if ch.is_uppercase() && prev_lower && !cur.is_empty() {
            words.push(std::mem::take(&mut cur));
        }
        prev_lower = ch.is_lowercase() || ch.is_ascii_digit();
        cur.push(ch);
    }
    if !cur.is_empty() {
        words.push(cur);
    }
    words.iter().filter_map(|w| w.chars().next()).flat_map(char::to_uppercase).take(max.max(1)).collect()
}

fn previews(image: &Path, sizes: &[[u32; 2]], out: &Path) -> Result<Vec<PathBuf>> {
    let img = image::open(image).map_err(|e| Error::Other(format!("{}: {e}", image.display())))?;
    mkdir(out)?;
    let mut written = Vec::new();
    for [w, h] in sizes {
        let path = out.join(format!("preview-{w}x{h}.png"));
        img.resize_exact(*w, *h, image::imageops::FilterType::Lanczos3)
            .save(&path)
            .map_err(|e| Error::Other(format!("{}: {e}", path.display())))?;
        written.push(path);
    }
    Ok(written)
}

pub fn run(recipe: &Recipe, opts: &RunOptions) -> Result<JobReport> {
    let job_dir = match &opts.job_dir {
        Some(d) => d.clone(),
        None => new_job_dir(&recipe.id)?,
    };
    let (in_dir, out_dir) = (job_dir.join("in"), job_dir.join("out"));
    if !opts.dry_run {
        mkdir(&in_dir)?;
        mkdir(&out_dir)?;
    }
    let inputs = prepare_inputs(recipe, &opts.inputs, &in_dir, opts.dry_run)?;

    let mut vars: BTreeMap<String, String> = BTreeMap::new();
    vars.insert("job.dir".into(), job_dir.display().to_string());
    vars.insert("job.in".into(), in_dir.display().to_string());
    vars.insert("job.out".into(), out_dir.display().to_string());
    vars.insert("recipe.dir".into(), recipe.base_dir().display().to_string());
    for (k, v) in recipe.preset_values()? {
        vars.insert(format!("preset.{k}"), v);
    }
    for (k, v) in &inputs {
        vars.insert(format!("inputs.{k}"), v.clone());
    }

    let default_timeout = Duration::from_secs(recipe.step_timeout_secs);
    let found = if opts.dry_run {
        BTreeMap::new()
    } else {
        {
            let needed: std::collections::BTreeSet<String> = recipe
                .steps
                .iter()
                .filter(|s| when_holds(s.when.as_deref(), &vars))
                .filter_map(|s| s.cli.as_ref().map(|c| c.app.clone()))
                .collect();
            preflight(recipe, Duration::from_secs(30), &opts.app_overrides, Some(&needed))?
        }
    };
    let app_uses: BTreeMap<String, AppUse> =
        found.iter().map(|(id, (cli, v))| (id.clone(), AppUse { cli: cli.path.clone(), version: v.clone() })).collect();

    let mut report = JobReport {
        recipe: recipe.id.clone(),
        recipe_version: recipe.version,
        job_dir: job_dir.clone(),
        dry_run: opts.dry_run,
        apps: app_uses,
        inputs,
        steps: Vec::new(),
        verify: Vec::new(),
    };

    for step in &recipe.steps {
        let started = std::time::Instant::now();
        let mut record = StepRecord { id: step.id.clone(), skipped: false, command: None, exit_code: None, millis: 0, outputs: vec![] };
        if !when_holds(step.when.as_deref(), &vars) {
            record.skipped = true;
            report.steps.push(record);
            continue;
        }
        if let Some(cli) = &step.cli {
            let args = cli.args.iter().map(|a| render(a, &vars)).collect::<Result<Vec<_>>>()?;
            let program = match found.get(&cli.app) {
                Some((c, _)) => c.path.clone(),
                None => PathBuf::from(format!("{}-cli", cli.app)),
            };
            let mut shown = vec![program.display().to_string()];
            shown.extend(args.iter().cloned());
            record.command = Some(shown);
            if !opts.dry_run {
                let timeout = step.timeout_secs.map(Duration::from_secs).unwrap_or(default_timeout);
                let out = apps::exec(&program, &args, Some(&job_dir), timeout)?;
                record.exit_code = out.code;
                if !out.ok() {
                    let message = if out.timed_out {
                        format!("stopped after {}s", timeout.as_secs())
                    } else {
                        format!("exit {:?}: {}", out.code, tail(&out.stderr, 5))
                    };
                    let version = found.get(&cli.app).map(|(_, v)| v.clone()).unwrap_or_default();
                    write_json(
                        &job_dir.join("failure.json"),
                        &json!({
                            "recipe": recipe.id,
                            "recipe_version": recipe.version,
                            "step": step.id,
                            "app": cli.app,
                            "app_version": version,
                            "command": record.command,
                            "exit_code": out.code,
                            "timed_out": out.timed_out,
                            "stdout_tail": tail(&out.stdout, 40),
                            "stderr_tail": tail(&out.stderr, 40),
                            "os": std::env::consts::OS,
                            "arch": std::env::consts::ARCH,
                            "job_dir": job_dir,
                        }),
                    )?;
                    report.steps.push(record);
                    write_json(&job_dir.join("job.json"), &report)?;
                    return Err(Error::Step { step: step.id.clone(), app: cli.app.clone(), message });
                }
            }
        } else if let Some(b) = &step.builtin {
            let prefix = format!("steps.{}.", step.id);
            match b {
                Builtin::CoverFit { image, width, height } => {
                    let path = PathBuf::from(render(image, &vars)?);
                    let values: Vec<(String, String)> = if opts.dry_run && !path.is_file() {
                        vec![
                            ("width".into(), width.to_string()),
                            ("height".into(), height.to_string()),
                            ("x".into(), "0".into()),
                            ("y".into(), "0".into()),
                        ]
                    } else {
                        cover_fit(&path, *width, *height)?.to_vec()
                    };
                    for (k, v) in values {
                        vars.insert(format!("{prefix}{k}"), v);
                    }
                }
                Builtin::Monogram { text, prefer, max } => {
                    let preferred = prefer.as_deref().map(|p| render(p, &vars)).transpose()?.filter(|p| !p.trim().is_empty());
                    let letters = match preferred {
                        Some(p) => p.trim().to_string(),
                        None => monogram(&render(text, &vars)?, *max),
                    };
                    vars.insert(format!("{prefix}letters"), letters);
                }
                Builtin::PackIcons { master, targets, out } => {
                    let targets_path = recipe.base_dir().join(render(targets, &vars)?);
                    let targets = pack::Targets::load(&targets_path)?;
                    if !opts.dry_run {
                        record.outputs = pack::pack(Path::new(&render(master, &vars)?), &targets, Path::new(&render(out, &vars)?))?;
                    }
                }
                Builtin::Previews { image, sizes, out } => {
                    if !opts.dry_run {
                        record.outputs = previews(Path::new(&render(image, &vars)?), sizes, Path::new(&render(out, &vars)?))?;
                    }
                }
            }
        }
        record.millis = started.elapsed().as_millis();
        report.steps.push(record);
    }

    if !opts.dry_run {
        for rule in &recipe.verify {
            let path = PathBuf::from(render(&rule.file, &vars)?);
            report.verify.push(verify::check(&path, rule));
        }
        write_json(&job_dir.join("job.json"), &report)?;
        let failed: Vec<String> =
            report.verify.iter().filter(|o| !o.ok).map(|o| format!("{}: {}", o.file.display(), o.problems.join("; "))).collect();
        if !failed.is_empty() {
            return Err(Error::Verify(failed.join("\n")));
        }
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn monograms_take_word_initials() {
        assert_eq!(monogram("PhotoCraft", 2), "PC");
        assert_eq!(monogram("my cool app", 2), "MC");
        assert_eq!(monogram("storycraft", 2), "S");
        assert_eq!(monogram("film-craft studio", 3), "FCS");
    }
}
