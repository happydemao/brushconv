//! GIMP's `.gbr`, version 2: the header, the name and the grey image.

use procraft_engine::brush::Brush;

use crate::Result;
use crate::images;

/// `brush` as a `.gbr` file.
pub fn write(brush: &Brush) -> Result<Vec<u8>> {
    let (side, pixels) = images::tip_gray(brush);
    let side = side.max(1);
    let name = brush.name.trim();
    let name = name.as_bytes();
    let header = 28u32.saturating_add(name.len() as u32).saturating_add(1);
    let percent = (brush.spacing * 100.0).round().clamp(1.0, 1000.0) as u32;
    let mut out = Vec::with_capacity(header as usize + pixels.len());
    out.extend_from_slice(&header.to_be_bytes());
    out.extend_from_slice(&2u32.to_be_bytes());
    out.extend_from_slice(&side.to_be_bytes());
    out.extend_from_slice(&side.to_be_bytes());
    out.extend_from_slice(&1u32.to_be_bytes());
    out.extend_from_slice(b"GIMP");
    out.extend_from_slice(&percent.to_be_bytes());
    out.extend_from_slice(name);
    out.push(0);
    out.extend_from_slice(&pixels);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_a_file_the_engine_reads() {
        let brush = Brush { name: "Ink".to_owned(), spacing: 0.1, ..Brush::round_pen() };
        let bytes = write(&brush).expect("write");
        let tips = procraft_engine::gbr::read(&bytes).expect("read");
        assert_eq!(tips.first().map(|t| t.name.as_str()), Some("Ink"));
        assert_eq!(tips.first().map(|t| t.width), Some(256));
    }
}
