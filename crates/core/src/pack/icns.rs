//! macOS `.icns` with PNG entries, the set `iconutil` writes from an `.iconset`.

/// Entry types by pixel size; 32, 64, 256, 512 and 1024 also serve as the @2x of the size below.
pub const ENTRIES: &[(&[u8; 4], u32)] = &[
    (b"icp4", 16),
    (b"icp5", 32),
    (b"ic11", 32),
    (b"ic12", 64),
    (b"ic07", 128),
    (b"ic08", 256),
    (b"ic13", 256),
    (b"ic09", 512),
    (b"ic14", 512),
    (b"ic10", 1024),
];

/// The pixel sizes an `.icns` needs.
pub fn sizes() -> Vec<u32> {
    let mut s: Vec<u32> = ENTRIES.iter().map(|(_, px)| *px).collect();
    s.dedup();
    s
}

/// `png_for(size)` gives the PNG for each size in [`sizes`].
pub fn write(png_for: impl Fn(u32) -> Option<Vec<u8>>) -> Option<Vec<u8>> {
    let mut body = Vec::new();
    for (kind, px) in ENTRIES {
        let png = png_for(*px)?;
        body.extend_from_slice(*kind);
        body.extend_from_slice(&((png.len() + 8) as u32).to_be_bytes());
        body.extend_from_slice(&png);
    }
    let mut out = Vec::with_capacity(body.len() + 8);
    out.extend_from_slice(b"icns");
    out.extend_from_slice(&((body.len() + 8) as u32).to_be_bytes());
    out.extend_from_slice(&body);
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_entry_is_written_with_its_length() -> Result<(), String> {
        let icns = write(|px| Some(vec![0u8; px as usize % 7 + 1])).ok_or("write failed")?;
        assert_eq!(&icns[..4], b"icns");
        assert_eq!(u32::from_be_bytes([icns[4], icns[5], icns[6], icns[7]]) as usize, icns.len());
        assert_eq!(crate::verify::container_images(&icns, crate::verify::Format::Icns), Some(ENTRIES.len()));
        assert_eq!(sizes(), vec![16, 32, 64, 128, 256, 512, 1024]);
        Ok(())
    }
}
