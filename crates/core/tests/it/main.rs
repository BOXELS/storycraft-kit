//! End-to-end recipe runs against fake app CLIs (shell scripts that log their arguments and write
//! fixture outputs). Live runs against real apps belong in `tests/live` and need the Mac.
#![cfg(unix)]

use std::collections::BTreeMap;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use storycraft_core::engine::{self, RunOptions};
use storycraft_core::recipe::Recipe;
use storycraft_core::{Error, bug};

type R = Result<(), Box<dyn std::error::Error>>;

fn recipes() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../recipes")
}

/// A fake `<app>-cli`: `--version`, `commands` (from `catalog`), and `body` for everything else.
/// Every call appends its arguments, one per line, then `--`, to `log`.
fn fake_cli(
    dir: &Path,
    name: &str,
    version: &str,
    catalog: &[&str],
    log: &Path,
    body: &str,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let path = dir.join(name);
    let script = format!(
        "#!/bin/sh\nfor a in \"$@\"; do printf '%s\\n' \"$a\" >> '{log}'; done\necho -- >> '{log}'\n\
         case \"$1\" in\n  --version) echo '{name} {version}'; exit 0;;\n  commands) printf '%s\\n' {cat}; exit 0;;\nesac\n{body}\n",
        log = log.display(),
        cat = catalog.iter().map(|c| format!("'{c}  description'")).collect::<Vec<_>>().join(" "),
    );
    std::fs::write(&path, script)?;
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))?;
    Ok(path)
}

/// The value after `flag` in each logged call.
fn logged_values(log: &Path, flag: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let text = std::fs::read_to_string(log)?;
    let lines: Vec<&str> = text.lines().collect();
    Ok(lines.windows(2).filter(|w| w[0] == flag).map(|w| w[1].to_string()).collect())
}

const PC_COMMANDS: &[&str] = &["image.imageSize", "image.crop", "type.create", "layer.layerStyle.stroke", "layer.layerStyle.dropShadow"];

#[test]
fn youtube_thumbnail_runs_end_to_end_with_a_fake_photocraft() -> R {
    let tmp = tempfile::tempdir()?;
    let photo = tmp.path().join("photo.png");
    image::RgbImage::from_pixel(1920, 1200, image::Rgb([200, 80, 40])).save(&photo)?;
    let fixture_jpg = tmp.path().join("fixture.jpg");
    image::RgbImage::from_pixel(1280, 720, image::Rgb([10, 20, 30])).save(&fixture_jpg)?;
    let log = tmp.path().join("calls.log");
    let body = format!(
        "if [ \"$1\" = run ]; then\n  while [ $# -gt 0 ]; do if [ \"$1\" = --out ]; then out=\"$2\"; fi; shift; done\n  \
         {{ printf '8BPS'; head -c 2000 /dev/zero; }} > \"$out\"; echo '{{\"ok\":true}}'; exit 0\nfi\n\
         if [ \"$1\" = convert ]; then cp '{jpg}' \"$3\"; exit 0; fi\nexit 2",
        jpg = fixture_jpg.display()
    );
    let cli = fake_cli(tmp.path(), "photocraft-cli", "0.23.1", PC_COMMANDS, &log, &body)?;

    let recipe = Recipe::load(&recipes().join("youtube-thumbnail.yaml"))?;
    let job = tmp.path().join("job");
    let opts = RunOptions {
        inputs: BTreeMap::from([("image".into(), photo.display().to_string()), ("title".into(), "Big \"News\" Today".into())]),
        job_dir: Some(job.clone()),
        dry_run: false,
        app_overrides: BTreeMap::from([("photocraft".into(), cli)]),
    };
    let report = engine::run(&recipe, &opts)?;
    assert!(report.verify.iter().all(|v| v.ok), "{:?}", report.verify);
    assert_eq!(report.apps["photocraft"].version, "0.23.1");
    assert!(job.join("job.json").is_file() && job.join("out/previews/preview-320x180.png").is_file());

    let params = logged_values(&log, "--params")?;
    assert_eq!(params.len(), 5);
    for p in &params {
        serde_json::from_str::<serde_json::Value>(p).map_err(|e| format!("bad JSON params {p}: {e}"))?;
    }
    let size: serde_json::Value = serde_json::from_str(&params[0])?;
    assert_eq!((size["width"].as_u64(), size["height"].as_u64()), (Some(1280), Some(800)), "cover-fit of 1920x1200 into 1280x720");
    let crop: serde_json::Value = serde_json::from_str(&params[1])?;
    assert_eq!(crop["y"], 40);
    let text: serde_json::Value = serde_json::from_str(&params[2])?;
    assert_eq!(text["text"], "Big \"News\" Today");
    Ok(())
}

#[test]
fn icon_set_draws_in_a_fake_vectorcraft_then_packs_every_target() -> R {
    let tmp = tempfile::tempdir()?;
    let fixture_svg = tmp.path().join("fixture.svg");
    std::fs::write(
        &fixture_svg,
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="1024" height="1024"><rect width="1024" height="1024" fill="#2f5bff"/><circle cx="512" cy="512" r="300" fill="#fff"/></svg>"##,
    )?;
    let job = tmp.path().join("job");
    let log = tmp.path().join("calls.log");
    let body = format!(
        "if [ \"$1\" = run ]; then mkdir -p '{out}'; cp '{svg}' '{out}/master.svg'; exit 0; fi\nexit 2",
        out = job.join("out").display(),
        svg = fixture_svg.display()
    );
    let cli =
        fake_cli(tmp.path(), "vectorcraft-cli", "0.24.0", &["file.new", "shape.rectangle", "text.create", "document.export"], &log, &body)?;

    let recipe = Recipe::load(&recipes().join("icon-set.yaml"))?;
    let opts = RunOptions {
        inputs: BTreeMap::from([("name".into(), "StoryCraft Kit".into())]),
        job_dir: Some(job.clone()),
        dry_run: false,
        app_overrides: BTreeMap::from([("vectorcraft".into(), cli)]),
    };
    let report = engine::run(&recipe, &opts)?;
    assert!(report.verify.iter().all(|v| v.ok), "{:?}", report.verify);
    let skipped: Vec<&str> = report.steps.iter().filter(|s| s.skipped).map(|s| s.id.as_str()).collect();
    assert_eq!(skipped, ["pack-given"]);
    let text = logged_values(&log, "--params")?.into_iter().find(|p| p.contains("\"text\"")).ok_or("no text.create params")?;
    let text: serde_json::Value = serde_json::from_str(&text)?;
    assert_eq!(text["text"], "SC", "camel case splits StoryCraft into Story + Craft");
    assert!(job.join("out/icons/contact-sheet.png").is_file());
    assert!(job.join("out/icons/macos/AppIcon.iconset/icon_512x512@2x.png").is_file());
    Ok(())
}

#[test]
fn icon_set_with_a_given_master_needs_no_app_step() -> R {
    let tmp = tempfile::tempdir()?;
    let master = tmp.path().join("mine.svg");
    std::fs::write(
        &master,
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"><rect width="10" height="10" fill="#0a0"/></svg>"##,
    )?;
    let recipe = Recipe::load(&recipes().join("icon-set.yaml"))?;
    let opts = RunOptions {
        inputs: BTreeMap::from([("name".into(), "X".into()), ("master".into(), master.display().to_string())]),
        job_dir: Some(tmp.path().join("job")),
        dry_run: false,
        app_overrides: BTreeMap::new(),
    };
    let report = engine::run(&recipe, &opts)?;
    assert!(report.steps.iter().any(|s| s.id == "draw" && s.skipped));
    assert!(report.apps.is_empty(), "VectorCraft is not needed when a master is given");
    Ok(())
}

#[test]
fn a_failing_step_leaves_failure_json_and_a_bug_draft() -> R {
    let tmp = tempfile::tempdir()?;
    let photo = tmp.path().join("photo.jpg");
    image::RgbImage::from_pixel(1280, 720, image::Rgb([1, 2, 3])).save(&photo)?;
    let log = tmp.path().join("calls.log");
    let cli = fake_cli(tmp.path(), "photocraft-cli", "0.23.1", PC_COMMANDS, &log, "echo 'type.create: unknown font' >&2; exit 1")?;
    let recipe = Recipe::load(&recipes().join("youtube-thumbnail.yaml"))?;
    let job = tmp.path().join("job");
    let opts = RunOptions {
        inputs: BTreeMap::from([("image".into(), photo.display().to_string()), ("title".into(), "Hi".into())]),
        job_dir: Some(job.clone()),
        dry_run: false,
        app_overrides: BTreeMap::from([("photocraft".into(), cli)]),
    };
    let err = engine::run(&recipe, &opts).err().ok_or("expected failure")?;
    assert!(matches!(err, Error::Step { ref step, .. } if step == "compose"), "{err}");
    let draft = bug::draft(&job)?;
    assert_eq!(draft.repo, "storytold/photocraft");
    assert!(draft.body.contains("unknown font"));
    Ok(())
}

#[test]
fn preflight_refuses_old_versions_and_missing_commands() -> R {
    let tmp = tempfile::tempdir()?;
    let log = tmp.path().join("calls.log");
    let recipe = Recipe::load(&recipes().join("youtube-thumbnail.yaml"))?;
    let old = fake_cli(tmp.path(), "photocraft-cli", "0.18.0", PC_COMMANDS, &log, "exit 0")?;
    let err = engine::preflight(&recipe, std::time::Duration::from_secs(10), &BTreeMap::from([("photocraft".into(), old)]), None)
        .err()
        .ok_or("expected too old")?;
    assert!(matches!(err, Error::AppTooOld { .. }), "{err}");
    let sub = tmp.path().join("b");
    std::fs::create_dir_all(&sub)?;
    let partial = fake_cli(&sub, "photocraft-cli", "0.24.0", &["image.crop"], &log, "exit 0")?;
    let err = engine::preflight(&recipe, std::time::Duration::from_secs(10), &BTreeMap::from([("photocraft".into(), partial)]), None)
        .err()
        .ok_or("expected missing")?;
    assert!(matches!(err, Error::CommandMissing { ref command, .. } if command == "image.imageSize"), "{err}");
    Ok(())
}

#[test]
fn dry_run_writes_nothing_and_shows_every_command() -> R {
    let tmp = tempfile::tempdir()?;
    let recipe = Recipe::load(&recipes().join("icon-set.yaml"))?;
    let job = tmp.path().join("job");
    let opts = RunOptions {
        inputs: BTreeMap::from([("name".into(), "PhotoCraft".into())]),
        job_dir: Some(job.clone()),
        dry_run: true,
        app_overrides: BTreeMap::new(),
    };
    let report = engine::run(&recipe, &opts)?;
    assert!(!job.exists());
    let draw = report.steps.iter().find(|s| s.id == "draw").and_then(|s| s.command.clone()).ok_or("no draw command")?;
    assert!(draw.iter().any(|a| a.contains("\"PC\"")), "{draw:?}");
    Ok(())
}
