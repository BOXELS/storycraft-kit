//! Windows `.ico` with PNG-compressed entries (Vista and later read these at every size).

/// `images`: `(size, png bytes)`, each at most 256 px square.
pub fn write(images: &[(u32, Vec<u8>)]) -> Vec<u8> {
    let count = images.len() as u16;
    let mut out = Vec::new();
    out.extend_from_slice(&[0, 0, 1, 0]);
    out.extend_from_slice(&count.to_le_bytes());
    let mut offset = 6 + 16 * images.len() as u32;
    for (size, png) in images {
        // 256 is written as 0.
        let dim = if *size >= 256 { 0 } else { *size as u8 };
        out.extend_from_slice(&[dim, dim, 0, 0]);
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&32u16.to_le_bytes());
        out.extend_from_slice(&(png.len() as u32).to_le_bytes());
        out.extend_from_slice(&offset.to_le_bytes());
        offset += png.len() as u32;
    }
    for (_, png) in images {
        out.extend_from_slice(png);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn directory_points_at_each_payload() {
        let ico = write(&[(16, vec![1, 2, 3]), (256, vec![4, 5])]);
        assert_eq!(&ico[..6], &[0, 0, 1, 0, 2, 0]);
        assert_eq!(ico[6 + 16], 0, "256 px is stored as 0");
        let off = u32::from_le_bytes([ico[18], ico[19], ico[20], ico[21]]) as usize;
        assert_eq!(&ico[off..off + 3], &[1, 2, 3]);
        assert_eq!(crate::verify::container_images(&ico, crate::verify::Format::Ico), Some(2));
    }
}
