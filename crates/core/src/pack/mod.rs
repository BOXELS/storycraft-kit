//! Icon packing: one SVG master -> every platform file a targets list names.
//!
//! The master is drawn full-bleed (art to the edges, square). Shapes adapt it per platform:
//! - `full`: as drawn (Windows, web, Linux, Android adaptive foreground);
//! - `macos`: Apple's grid, a rounded tile 824/1024 of the canvas, transparent margin;
//! - `opaque`: flattened onto `background` with no alpha (App Store, iOS).
//!
//! Text in the master must be outlined (VectorCraft `outlineText`), so no fonts are needed here.

pub mod icns;
pub mod ico;

use std::collections::BTreeMap;
use std::io::Cursor;
use std::path::{Path, PathBuf};

use image::{ImageFormat, Rgba, RgbaImage, imageops};
use serde::Deserialize;

use crate::recipe::parse_hex_color;
use crate::{Error, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, serde::Serialize, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum Shape {
    Full,
    Macos,
    Opaque,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PngTarget {
    pub path: String,
    pub size: u32,
    #[serde(default = "full")]
    pub shape: Shape,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IcoTarget {
    pub path: String,
    pub sizes: Vec<u32>,
    #[serde(default = "full")]
    pub shape: Shape,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IcnsTarget {
    pub path: String,
    #[serde(default = "macos")]
    pub shape: Shape,
}

fn full() -> Shape {
    Shape::Full
}
fn macos() -> Shape {
    Shape::Macos
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Targets {
    #[serde(default)]
    pub pngs: Vec<PngTarget>,
    #[serde(default)]
    pub ico: Vec<IcoTarget>,
    #[serde(default)]
    pub icns: Vec<IcnsTarget>,
    /// Copies of the master SVG (web favicon, Linux scalable).
    #[serde(default)]
    pub svg: Vec<String>,
    /// Fill for `opaque` shapes.
    #[serde(default = "white")]
    pub background: String,
    /// Apple grid: the tile's share of the canvas, and its corner radius as a share of the tile.
    #[serde(default = "tile_scale")]
    pub macos_tile: f32,
    #[serde(default = "tile_radius")]
    pub macos_radius: f32,
    /// Light and dark backgrounds for the contact sheet; empty to skip it.
    #[serde(default = "sheet")]
    pub contact_sheet: Option<String>,
}

fn white() -> String {
    "#FFFFFF".into()
}
fn tile_scale() -> f32 {
    824.0 / 1024.0
}
fn tile_radius() -> f32 {
    0.2237
}
fn sheet() -> Option<String> {
    Some("contact-sheet.png".into())
}

impl Targets {
    pub fn load(path: &Path) -> Result<Self> {
        let text = std::fs::read_to_string(path).map_err(|e| Error::io(path, e))?;
        serde_yaml::from_str(&text).map_err(|e| Error::Recipe(format!("{}: {e}", path.display())))
    }
}

pub struct Master {
    tree: resvg::usvg::Tree,
}

impl Master {
    pub fn load(path: &Path) -> Result<Self> {
        let data = std::fs::read(path).map_err(|e| Error::io(path, e))?;
        Self::from_bytes(&data).map_err(|e| Error::Other(format!("{}: {e}", path.display())))
    }

    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        let tree = resvg::usvg::Tree::from_data(data, &resvg::usvg::Options::default()).map_err(|e| Error::Other(format!("SVG: {e}")))?;
        Ok(Self { tree })
    }

    /// The master scaled to `size`×`size` (non-square masters are centred).
    pub fn render(&self, size: u32) -> Result<RgbaImage> {
        let mut pixmap = resvg::tiny_skia::Pixmap::new(size, size).ok_or_else(|| Error::Other(format!("cannot render at {size} px")))?;
        let s = self.tree.size();
        let scale = (size as f32 / s.width()).min(size as f32 / s.height());
        let dx = (size as f32 - s.width() * scale) / 2.0;
        let dy = (size as f32 - s.height() * scale) / 2.0;
        let transform = resvg::tiny_skia::Transform::from_row(scale, 0.0, 0.0, scale, dx, dy);
        resvg::render(&self.tree, transform, &mut pixmap.as_mut());
        let mut img = RgbaImage::new(size, size);
        for (dst, src) in img.pixels_mut().zip(pixmap.pixels()) {
            let c = src.demultiply();
            *dst = Rgba([c.red(), c.green(), c.blue(), c.alpha()]);
        }
        Ok(img)
    }
}

/// Coverage of a pixel centre by a rounded square of side `side` and corner radius `r`.
fn rounded_coverage(x: f32, y: f32, side: f32, r: f32) -> f32 {
    let cx = x.clamp(r, side - r);
    let cy = y.clamp(r, side - r);
    let d = ((x - cx).powi(2) + (y - cy).powi(2)).sqrt();
    (r - d + 0.5).clamp(0.0, 1.0)
}

pub fn shaped(master: &Master, size: u32, shape: Shape, targets: &Targets) -> Result<RgbaImage> {
    match shape {
        Shape::Full => master.render(size),
        Shape::Opaque => {
            let art = master.render(size)?;
            let (r, g, b) = parse_hex_color(&targets.background)
                .ok_or_else(|| Error::Recipe(format!("background `{}` is not a colour", targets.background)))?;
            let mut out = RgbaImage::from_pixel(size, size, Rgba([r, g, b, 255]));
            imageops::overlay(&mut out, &art, 0, 0);
            Ok(out)
        }
        Shape::Macos => {
            let tile = ((size as f32) * targets.macos_tile).round().max(1.0) as u32;
            let mut art = master.render(tile)?;
            let radius = tile as f32 * targets.macos_radius;
            for (x, y, px) in art.enumerate_pixels_mut() {
                let cov = rounded_coverage(x as f32 + 0.5, y as f32 + 0.5, tile as f32, radius);
                px.0[3] = (f32::from(px.0[3]) * cov).round() as u8;
            }
            let mut out = RgbaImage::new(size, size);
            let off = i64::from((size - tile) / 2);
            imageops::overlay(&mut out, &art, off, off);
            Ok(out)
        }
    }
}

fn png_bytes(img: &RgbaImage, opaque: bool) -> Result<Vec<u8>> {
    let mut buf = Cursor::new(Vec::new());
    let res = if opaque {
        image::DynamicImage::ImageRgba8(img.clone()).to_rgb8().write_to(&mut buf, ImageFormat::Png)
    } else {
        img.write_to(&mut buf, ImageFormat::Png)
    };
    res.map_err(|e| Error::Other(format!("PNG encode: {e}")))?;
    Ok(buf.into_inner())
}

fn write_file(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| Error::io(dir, e))?;
    }
    std::fs::write(path, bytes).map_err(|e| Error::io(path, e))
}

/// Light and dark strips of every PNG up to 256 px, to eyeball the set at a glance.
fn contact_sheet(rendered: &BTreeMap<(Shape, u32), RgbaImage>) -> Option<RgbaImage> {
    let pick: Vec<&RgbaImage> = rendered.iter().filter(|((_, s), _)| *s <= 256).map(|(_, i)| i).collect();
    if pick.is_empty() {
        return None;
    }
    let pad = 16u32;
    let width = pick.iter().map(|i| i.width() + pad).sum::<u32>() + pad;
    let row = pick.iter().map(|i| i.height()).max().unwrap_or(0) + 2 * pad;
    let mut sheet = RgbaImage::new(width, row * 2);
    for (y, px) in sheet.enumerate_pixels_mut().map(|(_, y, p)| (y, p)) {
        *px = if y < row { Rgba([245, 245, 245, 255]) } else { Rgba([30, 30, 30, 255]) };
    }
    for band in 0..2u32 {
        let mut x = pad;
        for img in &pick {
            let y = band * row + pad + (row - 2 * pad - img.height()) / 2;
            imageops::overlay(&mut sheet, *img, i64::from(x), i64::from(y));
            x += img.width() + pad;
        }
    }
    Some(sheet)
}

/// Write every target under `out`; returns the files written.
pub fn pack(master_path: &Path, targets: &Targets, out: &Path) -> Result<Vec<PathBuf>> {
    let master = Master::load(master_path)?;
    let mut cache: BTreeMap<(Shape, u32), RgbaImage> = BTreeMap::new();
    let mut get = |shape: Shape, size: u32| -> Result<RgbaImage> {
        if let Some(img) = cache.get(&(shape, size)) {
            return Ok(img.clone());
        }
        let img = shaped(&master, size, shape, targets)?;
        cache.insert((shape, size), img.clone());
        Ok(img)
    };
    let mut written = Vec::new();
    for t in &targets.pngs {
        let path = out.join(&t.path);
        write_file(&path, &png_bytes(&get(t.shape, t.size)?, t.shape == Shape::Opaque)?)?;
        written.push(path);
    }
    for t in &targets.ico {
        let mut images = Vec::new();
        for &s in &t.sizes {
            if s > 256 {
                return Err(Error::Recipe(format!("{}: .ico sizes stop at 256, not {s}", t.path)));
            }
            images.push((s, png_bytes(&get(t.shape, s)?, false)?));
        }
        let path = out.join(&t.path);
        write_file(&path, &ico::write(&images))?;
        written.push(path);
    }
    for t in &targets.icns {
        let mut pngs = BTreeMap::new();
        for s in icns::sizes() {
            pngs.insert(s, png_bytes(&get(t.shape, s)?, false)?);
        }
        let bytes = icns::write(|s| pngs.get(&s).cloned()).ok_or_else(|| Error::Other("icns: a size is missing".into()))?;
        let path = out.join(&t.path);
        write_file(&path, &bytes)?;
        written.push(path);
    }
    for rel in &targets.svg {
        let path = out.join(rel);
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| Error::io(dir, e))?;
        }
        std::fs::copy(master_path, &path).map_err(|e| Error::io(&path, e))?;
        written.push(path);
    }
    if let Some(name) = &targets.contact_sheet {
        let full_only: BTreeMap<(Shape, u32), RgbaImage> = cache.into_iter().filter(|((sh, _), _)| *sh != Shape::Opaque).collect();
        if let Some(sheet) = contact_sheet(&full_only) {
            let path = out.join(name);
            write_file(&path, &png_bytes(&sheet, false)?)?;
            written.push(path);
        }
    }
    Ok(written)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SVG: &[u8] =
        br##"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64"><rect width="64" height="64" fill="#3060ff"/></svg>"##;

    fn targets() -> Result<Targets> {
        serde_yaml::from_str("{}").map_err(Error::from)
    }

    #[test]
    fn macos_shape_leaves_a_transparent_margin_and_rounds_corners() -> Result<()> {
        let m = Master::from_bytes(SVG)?;
        let t = targets()?;
        let img = shaped(&m, 128, Shape::Macos, &t)?;
        assert_eq!(img.get_pixel(2, 2).0[3], 0, "margin is transparent");
        assert_eq!(img.get_pixel(64, 64).0[3], 255, "centre is solid");
        let tile_left = (128 - (128.0 * t.macos_tile).round() as u32) / 2;
        assert!(img.get_pixel(tile_left, tile_left).0[3] < 64, "corner is rounded off");
        let opaque = shaped(&m, 32, Shape::Opaque, &t)?;
        assert!(opaque.pixels().all(|p| p.0[3] == 255));
        Ok(())
    }

    #[test]
    fn pack_writes_every_target() -> Result<()> {
        let dir = tempfile::tempdir().map_err(|e| Error::Other(e.to_string()))?;
        let master = dir.path().join("m.svg");
        std::fs::write(&master, SVG).map_err(|e| Error::io(&master, e))?;
        let t: Targets = serde_yaml::from_str(
            "pngs: [{path: a/16.png, size: 16}, {path: ios/1024.png, size: 1024, shape: opaque}]\nico: [{path: app.ico, sizes: [16, 32, 256]}]\nicns: [{path: App.icns}]\nsvg: [web/icon.svg]\n",
        )?;
        let out = dir.path().join("out");
        let files = pack(&master, &t, &out)?;
        assert_eq!(files.len(), 6, "{files:?}");
        let ios = image::open(out.join("ios/1024.png")).map_err(|e| Error::Other(e.to_string()))?;
        assert!(!ios.color().has_alpha());
        Ok(())
    }
}
