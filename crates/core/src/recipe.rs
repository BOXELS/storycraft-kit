//! Recipes: a workflow as data (`recipes/*.yaml`), run by [`crate::engine`].
//!
//! A recipe names its inputs, the app versions and command ids it needs, its steps, and what the
//! outputs must look like. Unknown keys are refused. Steps are either an app CLI call (`cli`) or a
//! built-in (`builtin`) for work no app does (cover-fit maths, icon packing, previews).
//!
//! Templating: `{{inputs.title}}`, `{{preset.font}}`, `{{job.in}}`, `{{job.out}}`, `{{steps.fit.width}}`.
//! Filters: `{{x|json}}` (a JSON string literal, for text inside `--params`), `{{x|rgb01}}`
//! (`#rrggbb` -> `[r, g, b]` floats 0–1, PhotoCraft's colour form). An unknown name is an error.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::{Error, Result, paths, verify};

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Recipe {
    pub id: String,
    pub version: u32,
    pub summary: String,
    #[serde(default)]
    pub requires: BTreeMap<String, Requirement>,
    #[serde(default)]
    pub inputs: BTreeMap<String, Input>,
    /// A flat YAML map of values, relative to the recipe file, exposed as `preset.*`.
    #[serde(default)]
    pub preset: Option<String>,
    /// Default seconds an app step may run before it is stopped.
    #[serde(default = "default_timeout")]
    pub step_timeout_secs: u64,
    pub steps: Vec<Step>,
    #[serde(default)]
    pub verify: Vec<verify::Rule>,
    #[serde(skip)]
    pub source: PathBuf,
}

fn default_timeout() -> u64 {
    300
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Requirement {
    #[serde(default)]
    pub min_version: Option<String>,
    /// Command ids the app must list in `<app>-cli commands`.
    #[serde(default)]
    pub commands: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum InputKind {
    File,
    Text,
    Color,
    Number,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Input {
    #[serde(rename = "type")]
    pub kind: InputKind,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub default: Option<String>,
    /// Accepted file extensions, lower case without the dot.
    #[serde(default)]
    pub extensions: Vec<String>,
    #[serde(default)]
    pub help: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Step {
    pub id: String,
    #[serde(default)]
    pub note: Option<String>,
    /// `inputs.master` runs the step only when that value is set and non-empty; `!inputs.master` only when it is not.
    #[serde(default)]
    pub when: Option<String>,
    #[serde(default)]
    pub cli: Option<CliStep>,
    #[serde(default)]
    pub builtin: Option<Builtin>,
    #[serde(default)]
    pub timeout_secs: Option<u64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CliStep {
    pub app: String,
    pub args: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "use", rename_all = "kebab-case", deny_unknown_fields)]
pub enum Builtin {
    /// Scale-and-crop maths so `image` covers `width`×`height`; sets `steps.<id>.{width,height,x,y}`.
    CoverFit { image: String, width: u32, height: u32 },
    /// Initials for an icon (`PhotoCraft` -> `PC`); sets `steps.<id>.letters`. A non-empty `prefer` is used as given.
    Monogram {
        text: String,
        #[serde(default)]
        prefer: Option<String>,
        #[serde(default = "two")]
        max: usize,
    },
    /// Rasterize an SVG master into every platform file a targets list names.
    PackIcons { master: String, targets: String, out: String },
    /// Downscaled copies of an image, to judge legibility at small sizes.
    Previews { image: String, sizes: Vec<[u32; 2]>, out: String },
}

fn two() -> usize {
    2
}

impl Recipe {
    pub fn parse(text: &str, source: &Path) -> Result<Self> {
        let mut r: Recipe = serde_yaml::from_str(text).map_err(|e| Error::Recipe(format!("{}: {e}", source.display())))?;
        r.source = source.to_path_buf();
        r.check()?;
        Ok(r)
    }

    pub fn load(path: &Path) -> Result<Self> {
        let text = std::fs::read_to_string(path).map_err(|e| Error::io(path, e))?;
        Self::parse(&text, path)
    }

    /// The folder the recipe lives in; `preset` and `targets` paths are relative to it.
    pub fn base_dir(&self) -> PathBuf {
        self.source.parent().map(Path::to_path_buf).unwrap_or_default()
    }

    fn check(&self) -> Result<()> {
        let bad = |m: String| Err(Error::Recipe(format!("{}: {m}", self.source.display())));
        let mut seen = std::collections::BTreeSet::new();
        for s in &self.steps {
            if !seen.insert(s.id.as_str()) {
                return bad(format!("two steps are called `{}`", s.id));
            }
            match (&s.cli, &s.builtin) {
                (Some(c), None) if !self.requires.contains_key(&c.app) => {
                    return bad(format!("step `{}` uses {} but `requires` does not name it", s.id, c.app));
                }
                (Some(_), None) | (None, Some(_)) => {}
                _ => return bad(format!("step `{}` needs exactly one of `cli` or `builtin`", s.id)),
            }
        }
        for (name, input) in &self.inputs {
            if input.required && input.default.is_some() {
                return bad(format!("input `{name}` is required and has a default; pick one"));
            }
        }
        Ok(())
    }

    pub fn preset_values(&self) -> Result<BTreeMap<String, String>> {
        let Some(rel) = &self.preset else { return Ok(BTreeMap::new()) };
        let path = self.base_dir().join(rel);
        let text = std::fs::read_to_string(&path).map_err(|e| Error::io(&path, e))?;
        let map: BTreeMap<String, serde_yaml::Value> = serde_yaml::from_str(&text)?;
        map.into_iter()
            .map(|(k, v)| match v {
                serde_yaml::Value::String(s) => Ok((k, s)),
                serde_yaml::Value::Number(n) => Ok((k, n.to_string())),
                serde_yaml::Value::Bool(b) => Ok((k, b.to_string())),
                _ => Err(Error::Recipe(format!("{}: preset `{k}` must be a string, number or boolean", path.display()))),
            })
            .collect()
    }
}

/// Find a recipe by path or by id in the recipes folder.
pub fn resolve(name: &str) -> Result<PathBuf> {
    let direct = PathBuf::from(name);
    if direct.is_file() {
        return Ok(direct);
    }
    let dir = paths::recipes_dir().ok_or_else(|| Error::Recipe("no recipes folder; set SC_RECIPES_DIR".into()))?;
    let by_id = dir.join(format!("{name}.yaml"));
    if by_id.is_file() {
        return Ok(by_id);
    }
    Err(Error::Recipe(format!("no recipe `{name}` (looked for {} and {})", direct.display(), by_id.display())))
}

/// Every recipe in a folder (`*.yaml`, not presets), sorted by id; unreadable ones are reported, not skipped silently.
pub fn list(dir: &Path) -> Result<Vec<Result<Recipe>>> {
    let mut paths: Vec<PathBuf> = std::fs::read_dir(dir)
        .map_err(|e| Error::io(dir, e))?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_file() && p.extension().is_some_and(|e| e == "yaml" || e == "yml"))
        .collect();
    paths.sort();
    Ok(paths.iter().map(|p| Recipe::load(p)).collect())
}

/// `#rgb` / `#rrggbb` -> `(r, g, b)` bytes.
pub fn parse_hex_color(s: &str) -> Option<(u8, u8, u8)> {
    let hex = s.trim().strip_prefix('#')?;
    let byte = |h: &str| u8::from_str_radix(h, 16).ok();
    match hex.len() {
        6 => Some((byte(&hex[0..2])?, byte(&hex[2..4])?, byte(&hex[4..6])?)),
        3 => {
            let d = |i: usize| byte(&hex[i..i + 1]).map(|v| v * 17);
            Some((d(0)?, d(1)?, d(2)?))
        }
        _ => None,
    }
}

fn apply_filter(value: &str, filter: Option<&str>, expr: &str) -> Result<String> {
    match filter {
        None => Ok(value.to_string()),
        Some("json") => Ok(serde_json::to_string(value)?),
        Some("rgb01") => {
            let (r, g, b) = parse_hex_color(value).ok_or_else(|| Error::Recipe(format!("`{expr}`: `{value}` is not a #rrggbb colour")))?;
            let f = |c: u8| format!("{:.4}", f64::from(c) / 255.0);
            Ok(format!("[{}, {}, {}]", f(r), f(g), f(b)))
        }
        Some(other) => Err(Error::Recipe(format!("`{expr}`: unknown filter `{other}` (json, rgb01)"))),
    }
}

/// Replace every `{{name}}` / `{{name|filter}}` in `text` from `vars`.
pub fn render(text: &str, vars: &BTreeMap<String, String>) -> Result<String> {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find("{{") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        let Some(end) = after.find("}}") else {
            return Err(Error::Recipe(format!("unclosed `{{{{` in `{text}`")));
        };
        let expr = after[..end].trim();
        let (name, filter) = match expr.split_once('|') {
            Some((n, f)) => (n.trim(), Some(f.trim())),
            None => (expr, None),
        };
        let value = vars.get(name).ok_or_else(|| Error::Recipe(format!("`{{{{{expr}}}}}`: nothing called `{name}`")))?;
        out.push_str(&apply_filter(value, filter, expr)?);
        rest = &after[end + 2..];
    }
    out.push_str(rest);
    Ok(out)
}

/// Whether a step's `when` holds.
pub fn when_holds(cond: Option<&str>, vars: &BTreeMap<String, String>) -> bool {
    let Some(cond) = cond.map(str::trim) else { return true };
    let (negate, name) = match cond.strip_prefix('!') {
        Some(n) => (true, n.trim()),
        None => (false, cond),
    };
    let set = vars.get(name).is_some_and(|v| !v.trim().is_empty());
    set != negate
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vars(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
    }

    #[test]
    fn templating_escapes_text_for_json_params() -> Result<()> {
        let v = vars(&[("inputs.title", "He said \"hi\""), ("preset.color", "#ff8000")]);
        let s = render(r#"{"text":{{inputs.title|json}},"color":{{preset.color|rgb01}}}"#, &v)?;
        let parsed: serde_json::Value = serde_json::from_str(&s)?;
        assert_eq!(parsed["text"], "He said \"hi\"");
        assert_eq!(parsed["color"][0], 1.0);
        assert!(render("{{nope}}", &v).is_err());
        assert!(render("{{inputs.title|shout}}", &v).is_err());
        Ok(())
    }

    #[test]
    fn when_tests_presence() {
        let v = vars(&[("inputs.master", "/x.svg"), ("inputs.empty", "")]);
        assert!(when_holds(Some("inputs.master"), &v));
        assert!(!when_holds(Some("!inputs.master"), &v));
        assert!(when_holds(Some("!inputs.empty"), &v));
        assert!(when_holds(None, &v));
    }

    #[test]
    fn unknown_keys_and_unlisted_apps_are_refused() {
        let base = "id: x\nversion: 1\nsummary: s\nsteps:\n";
        let typo = format!("{base}  - id: a\n    cli: {{app: photocraft, args: []}}\n    timeout: 3\n");
        assert!(Recipe::parse(&typo, Path::new("t.yaml")).is_err());
        let unlisted = format!("{base}  - id: a\n    cli: {{app: photocraft, args: []}}\n");
        assert!(Recipe::parse(&unlisted, Path::new("t.yaml")).is_err());
        let both = format!("{base}  - id: a\n");
        assert!(Recipe::parse(&both, Path::new("t.yaml")).is_err());
    }

    #[test]
    fn shipped_recipes_parse() -> Result<()> {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../recipes");
        let all = list(&dir)?;
        assert!(all.len() >= 2, "expected the shipped recipes in {}", dir.display());
        for r in all {
            let r = r?;
            r.preset_values()?;
        }
        Ok(())
    }
}
