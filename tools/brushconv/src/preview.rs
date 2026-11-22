//! A stroke preview drawn without the GPU, for files that need a thumbnail.

use procraft_engine::brush::Brush;
use procraft_engine::preview;

use crate::Result;
use crate::images;

/// A preview of `brush`'s sample stroke: black on transparent, the size ProCraft's previews are.
pub fn stroke(brush: &Brush) -> Result<Vec<u8>> {
    let (width, height, rgba) = stroke_rgba(brush)?;
    images::rgba_png(width, height, &rgba)
}

/// [`stroke`]'s pixels: RGBA8, straight alpha.
pub fn stroke_rgba(brush: &Brush) -> Result<(u32, u32, Vec<u8>)> {
    let (width, height) = (preview::CANVAS.0 as u32, preview::CANVAS.1 as u32);
    let mut alpha = vec![0f32; width as usize * height as usize];
    let samples = preview::samples(brush);
    if samples.is_empty() {
        return Ok((width, height, vec![0; width as usize * height as usize * 4]));
    }
    let (tip_side, tip) = images::tip_gray(brush);
    let diameter = preview::diameter(brush).clamp(1.0, 2000.0);
    let spacing = brush.spacing.clamp(0.02, 2.0);
    let step = (spacing * diameter).max(0.75);
    // how much pressure thins the stamp and lowers its opacity
    let pressure_size = brush.pressure_size.clamp(0.0, 1.0);
    let pressure_opacity = brush.pressure_opacity.clamp(0.0, 1.0);
    let paint = |alpha: &mut [f32], cx: f32, cy: f32, size: f32, opacity: f32| {
        if size <= 0.0 || opacity <= 0.0 {
            return;
        }
        stamp(alpha, width, height, &tip, tip_side, cx, cy, size, opacity);
    };
    let mut carry = 0.0;
    let mut last = samples.first().map(|s| (s.pos.x, s.pos.y, s.pressure));
    for next in samples.iter().skip(1) {
        let (px, py, pp) = last.unwrap_or((next.pos.x, next.pos.y, next.pressure));
        let (dx, dy) = (next.pos.x - px, next.pos.y - py);
        let length = (dx * dx + dy * dy).sqrt();
        if length.is_finite() && length > 0.0 {
            let steps = ((length + carry) / step).floor();
            let count = steps.min(20_000.0) as i32;
            for i in 0..count {
                let along = ((i as f32 * step) + (step - carry)) / length;
                let along = along.clamp(0.0, 1.0);
                let (x, y) = (px + dx * along, py + dy * along);
                let pressure = pp + (next.pressure - pp) * along;
                let size = diameter * (1.0 - pressure_size * (1.0 - pressure)).max(0.05);
                let opacity = opacity_at(brush, pressure, pressure_opacity);
                paint(&mut alpha, x, y, size, opacity);
            }
            carry = (carry + length) % step;
        }
        last = Some((next.pos.x, next.pos.y, next.pressure));
    }
    let mut rgba = Vec::with_capacity(alpha.len() * 4);
    for a in alpha {
        rgba.extend_from_slice(&[0, 0, 0, (a.clamp(0.0, 1.0) * 255.0).round() as u8]);
    }
    Ok((width, height, rgba))
}

fn opacity_at(brush: &Brush, pressure: f32, pressure_opacity: f32) -> f32 {
    let least = brush.min_opacity.clamp(0.0, 1.0);
    let most = brush.max_opacity.clamp(0.0, 1.0).max(least);
    let base = least + (most - least) * pressure.clamp(0.0, 1.0);
    (base * (1.0 - pressure_opacity * (1.0 - pressure))).clamp(0.0, 1.0)
}

/// Lays one stamp of `tip` (`side` square) centred at `cx`, `cy`, `size` across, its coverage
/// times `opacity`, onto `alpha`.
fn stamp(alpha: &mut [f32], width: u32, height: u32, tip: &[u8], side: u32, cx: f32, cy: f32, size: f32, opacity: f32) {
    let half = size * 0.5;
    let left = (cx - half).floor().max(0.0) as i64;
    let top = (cy - half).floor().max(0.0) as i64;
    let right = ((cx + half).ceil() as i64).min(i64::from(width));
    let bottom = ((cy + half).ceil() as i64).min(i64::from(height));
    if right <= left || bottom <= top {
        return;
    }
    let tip_side = side.max(1) as f32;
    for y in top..bottom {
        for x in left..right {
            let u = ((x as f32 + 0.5 - (cx - half)) / size * tip_side - 0.5).clamp(0.0, tip_side - 1.0);
            let v = ((y as f32 + 0.5 - (cy - half)) / size * tip_side - 0.5).clamp(0.0, tip_side - 1.0);
            let coverage = sample(tip, side, u, v);
            if coverage <= 0.0 {
                continue;
            }
            let at = y as usize * width as usize + x as usize;
            if let Some(a) = alpha.get_mut(at) {
                let src = coverage * opacity;
                *a = src + *a * (1.0 - src);
            }
        }
    }
}

/// A point of a square image, bilinear, 0..1.
fn sample(pixels: &[u8], side: u32, x: f32, y: f32) -> f32 {
    let n = side as usize;
    if n == 0 {
        return 0.0;
    }
    let (x0, y0) = (x.floor().max(0.0) as usize, y.floor().max(0.0) as usize);
    let (x1, y1) = ((x0 + 1).min(n - 1), (y0 + 1).min(n - 1));
    let (fx, fy) = (x - x0 as f32, y - y0 as f32);
    let at = |x: usize, y: usize| f32::from(pixels.get(y * n + x).copied().unwrap_or(0)) / 255.0;
    let top = at(x0, y0) * (1.0 - fx) + at(x1, y0) * fx;
    let bottom = at(x0, y1) * (1.0 - fx) + at(x1, y1) * fx;
    top * (1.0 - fy) + bottom * fy
}
