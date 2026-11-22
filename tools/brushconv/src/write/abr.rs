//! Photoshop's `.abr`, version 2: sampled tips with their names and spacing.
//!
//! Version 2 is the oldest layout Photoshop and every other app reads (see
//! `procraft_engine::abr`): a 16-bit version, a count, then each brush's kind (2, sampled), its
//! body's length and its body.

use procraft_engine::brush::Brush;

use crate::Result;
use crate::images;

/// The most side an ABR tip can have (Photoshop's own limit).
const MOST_SIDE: u32 = 5000;

/// `brushes` as one `.abr` file.
pub fn write(brushes: &[Brush]) -> Result<Vec<u8>> {
    let count = u16::try_from(brushes.len().min(usize::from(u16::MAX))).unwrap_or(u16::MAX);
    let mut out = Vec::new();
    out.extend_from_slice(&2u16.to_be_bytes());
    out.extend_from_slice(&count.to_be_bytes());
    for brush in brushes.iter().take(usize::from(count)) {
        let body = body(brush)?;
        out.extend_from_slice(&2u16.to_be_bytes());
        out.extend_from_slice(&(body.len() as u32).to_be_bytes());
        out.extend_from_slice(&body);
    }
    Ok(out)
}

/// One brush's block body.
fn body(brush: &Brush) -> Result<Vec<u8>> {
    let (side, pixels) = images::tip_gray(brush);
    let side = side.clamp(1, MOST_SIDE);
    let pixels = fit(side, &pixels);
    let mut out = Vec::new();
    // a word Photoshop keeps before the spacing (not read back)
    out.extend_from_slice(&0u32.to_be_bytes());
    let percent = (brush.spacing * 100.0).round().clamp(1.0, 1000.0);
    out.extend_from_slice(&(percent as u16).to_be_bytes());
    // the name, as UTF-16BE units and a last NUL, its length first
    let units: Vec<u16> = brush.name.encode_utf16().filter(|u| *u != 0).collect();
    let units = if units.is_empty() { "Brush".encode_utf16().collect() } else { units };
    out.extend_from_slice(&((units.len() + 1) as u32).to_be_bytes());
    for unit in &units {
        out.extend_from_slice(&unit.to_be_bytes());
    }
    out.extend_from_slice(&0u16.to_be_bytes());
    // antialiasing on
    out.push(1);
    // a short bounds rectangle, then the one used
    for value in [0i16, 0, side as i16, side as i16] {
        out.extend_from_slice(&value.to_be_bytes());
    }
    for value in [0i32, 0, side as i32, side as i32] {
        out.extend_from_slice(&value.to_be_bytes());
    }
    out.extend_from_slice(&8u16.to_be_bytes());
    // uncompressed
    out.push(0);
    out.extend_from_slice(&pixels);
    Ok(out)
}

/// `pixels` (a `from` square) resampled to `to` square.
fn fit(to: u32, pixels: &[u8]) -> Vec<u8> {
    let from = (pixels.len() as f64).sqrt().round() as u32;
    if from == to || from == 0 {
        return pixels.to_vec();
    }
    let n = to as usize;
    let source = from as f32;
    let mut out = Vec::with_capacity(n * n);
    for y in 0..n {
        for x in 0..n {
            let sx = (x as f32 + 0.5) / n as f32 * source - 0.5;
            let sy = (y as f32 + 0.5) / n as f32 * source - 0.5;
            out.push(sample(pixels, from, sx, sy));
        }
    }
    out
}

fn sample(pixels: &[u8], side: u32, x: f32, y: f32) -> u8 {
    let n = side as usize;
    if n == 0 {
        return 0;
    }
    let (x0, y0) = (x.floor().max(0.0) as usize, y.floor().max(0.0) as usize);
    let (x1, y1) = ((x0 + 1).min(n - 1), (y0 + 1).min(n - 1));
    let (fx, fy) = (x - x0 as f32, y - y0 as f32);
    let at = |x: usize, y: usize| f32::from(pixels.get(y * n + x).copied().unwrap_or(0));
    let top = at(x0, y0) * (1.0 - fx) + at(x1, y0) * fx;
    let bottom = at(x0, y1) * (1.0 - fx) + at(x1, y1) * fx;
    (top * (1.0 - fy) + bottom * fy).round().clamp(0.0, 255.0) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_a_file_the_engine_reads() {
        let brush = Brush { name: "Soft Round".to_owned(), spacing: 0.25, ..Brush::round_pen() };
        let bytes = write(&[brush]).expect("write");
        let tips = procraft_engine::abr::read(&bytes).expect("read");
        assert_eq!(tips.len(), 1);
        assert_eq!(tips.first().map(|t| t.name.as_str()), Some("Soft Round"));
        let spacing = tips.first().and_then(|t| t.spacing);
        assert!((spacing.unwrap_or(0.0) - 0.25).abs() < 0.001);
    }
}
