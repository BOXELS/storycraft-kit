//! Checking outputs against what a recipe promised: format, pixel size, bytes, alpha.
//!
//! An app saying it succeeded is not enough; every recipe output is opened and measured here.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Format {
    Png,
    Jpeg,
    Svg,
    Ico,
    Icns,
    Psd,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Rule {
    /// Templated path (`{{job.out}}/thumbnail.jpg`).
    pub file: String,
    #[serde(default)]
    pub format: Option<Format>,
    #[serde(default)]
    pub width: Option<u32>,
    #[serde(default)]
    pub height: Option<u32>,
    #[serde(default)]
    pub max_bytes: Option<u64>,
    #[serde(default)]
    pub min_bytes: Option<u64>,
    /// `true`: must have an alpha channel; `false`: must not (App Store icons).
    #[serde(default)]
    pub alpha: Option<bool>,
    /// For `.ico` / `.icns`: the least number of images inside.
    #[serde(default)]
    pub min_images: Option<usize>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Outcome {
    pub file: PathBuf,
    pub ok: bool,
    pub problems: Vec<String>,
}

pub fn sniff(bytes: &[u8]) -> Option<Format> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some(Format::Png)
    } else if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some(Format::Jpeg)
    } else if bytes.starts_with(b"8BPS") {
        Some(Format::Psd)
    } else if bytes.starts_with(b"icns") {
        Some(Format::Icns)
    } else if bytes.starts_with(&[0, 0, 1, 0]) {
        Some(Format::Ico)
    } else {
        let head = String::from_utf8_lossy(&bytes[..bytes.len().min(512)]);
        head.contains("<svg").then_some(Format::Svg)
    }
}

/// How many images an `.ico` or `.icns` holds.
pub fn container_images(bytes: &[u8], format: Format) -> Option<usize> {
    match format {
        Format::Ico => bytes.get(4..6).map(|b| usize::from(u16::from_le_bytes([b[0], b[1]]))),
        Format::Icns => {
            let mut n = 0;
            let mut at = 8;
            while at + 8 <= bytes.len() {
                let len = u32::from_be_bytes([bytes[at + 4], bytes[at + 5], bytes[at + 6], bytes[at + 7]]) as usize;
                if len < 8 {
                    return None;
                }
                n += 1;
                at += len;
            }
            Some(n)
        }
        _ => None,
    }
}

pub fn check(path: &Path, rule: &Rule) -> Outcome {
    let mut problems = Vec::new();
    let bytes = match std::fs::read(path) {
        Ok(b) => b,
        Err(e) => {
            return Outcome { file: path.to_path_buf(), ok: false, problems: vec![format!("cannot read: {e}")] };
        }
    };
    let len = bytes.len() as u64;
    if let Some(max) = rule.max_bytes.filter(|m| len > *m) {
        problems.push(format!("{len} bytes, more than {max}"));
    }
    if let Some(min) = rule.min_bytes.filter(|m| len < *m) {
        problems.push(format!("{len} bytes, fewer than {min}"));
    }
    let found = sniff(&bytes);
    if let Some(want) = rule.format
        && found != Some(want)
    {
        problems.push(format!("format {found:?}, expected {want:?}"));
    }
    if matches!(found, Some(Format::Png | Format::Jpeg)) && (rule.width.is_some() || rule.height.is_some() || rule.alpha.is_some()) {
        match image::load_from_memory(&bytes) {
            Ok(img) => {
                if let Some(w) = rule.width.filter(|w| *w != img.width()) {
                    problems.push(format!("width {}, expected {w}", img.width()));
                }
                if let Some(h) = rule.height.filter(|h| *h != img.height()) {
                    problems.push(format!("height {}, expected {h}", img.height()));
                }
                if let Some(want) = rule.alpha {
                    let has = img.color().has_alpha();
                    if has != want {
                        problems.push(if want { "no alpha channel".to_string() } else { "has an alpha channel".to_string() });
                    }
                }
            }
            Err(e) => problems.push(format!("cannot decode: {e}")),
        }
    }
    if let (Some(min), Some(fmt)) = (rule.min_images, found) {
        match container_images(&bytes, fmt) {
            Some(n) if n >= min => {}
            Some(n) => problems.push(format!("{n} images inside, expected at least {min}")),
            None => problems.push("not an .ico/.icns container".into()),
        }
    }
    Outcome { file: path.to_path_buf(), ok: problems.is_empty(), problems }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes_formats_and_alpha_are_checked() -> std::io::Result<()> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("a.png");
        image::RgbImage::new(4, 3).save(&path).map_err(std::io::Error::other)?;
        let rule = |w, alpha| Rule {
            file: String::new(),
            format: Some(Format::Png),
            width: Some(w),
            height: Some(3),
            max_bytes: Some(10_000),
            min_bytes: None,
            alpha,
            min_images: None,
        };
        assert!(check(&path, &rule(4, Some(false))).ok);
        let bad = check(&path, &rule(5, Some(true)));
        assert_eq!(bad.problems.len(), 2, "{:?}", bad.problems);
        Ok(())
    }
}
