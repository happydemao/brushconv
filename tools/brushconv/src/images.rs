//! Pixel and PNG helpers shared by the writers.

use procraft_engine::brush::Brush;
use procraft_engine::textures;

use crate::preview;
use crate::{Error, Result};

/// The shape's grey image and its side: the brush's own picture at its sharpest, one of ProCraft's
/// stock shapes, or its round tip drawn from its hardness.
pub fn tip_gray(brush: &Brush) -> (u32, Vec<u8>) {
    if let Some(file) = &brush.shape_file
        && let Some(pixels) = &file.full.pixels
        && file.full.side >= textures::TEXTURE_SIZE as u32
        && pixels.len() == (file.full.side as usize).pow(2)
        && brush.shape.source.as_ref().is_none_or(|s| s.id == file.source)
    {
        return (file.full.side, pixels.to_vec());
    }
    if let Some(source) = &brush.shape.source {
        return (textures::TEXTURE_SIZE as u32, source.pixels.to_vec());
    }
    if let Some(image) = brush.shape.image.and_then(textures::shape_image) {
        return (textures::TEXTURE_SIZE as u32, image.pixels);
    }
    (textures::TEXTURE_SIZE as u32, round_tip(brush.hardness))
}

/// A round tip filling a square, its edge as soft as `hardness` says.
fn round_tip(hardness: f32) -> Vec<u8> {
    let n = textures::TEXTURE_SIZE;
    let hard = hardness.clamp(0.0, 1.0);
    let soft = (1.0 - hard).max(0.02);
    (0..n * n)
        .map(|i| {
            let (x, y) = ((i % n) as f32 + 0.5, (i / n) as f32 + 0.5);
            let r = ((x / n as f32 - 0.5).powi(2) + (y / n as f32 - 0.5).powi(2)).sqrt() * 2.0;
            let v = ((1.0 - r) / soft).clamp(0.0, 1.0);
            (v * v * (3.0 - 2.0 * v) * 255.0).round() as u8
        })
        .collect()
}

/// The grain's grey image, if the brush has one (its own, or one of ProCraft's stock grains).
pub fn grain_gray(brush: &Brush) -> Option<(u32, Vec<u8>)> {
    if let Some(file) = &brush.grain_file
        && let Some(full) = &file.full
        && let Some(pixels) = &full.pixels
        && full.side >= textures::TEXTURE_SIZE as u32
        && pixels.len() == (full.side as usize).pow(2)
    {
        return Some((full.side, pixels.to_vec()));
    }
    if let Some(source) = &brush.grain.source {
        return Some((textures::TEXTURE_SIZE as u32, source.pixels.to_vec()));
    }
    brush.grain.image.and_then(textures::grain_image).map(|g| (textures::TEXTURE_SIZE as u32, g.pixels))
}

/// `side`'s grey image boxed down to `max` a side, if it is bigger: what a writer whose format
/// takes smaller pictures than ours sends.
pub fn fit_gray(side: u32, pixels: &[u8], max: u32) -> (u32, Vec<u8>) {
    let n = side as usize;
    if max == 0 || n <= max as usize || pixels.len() != n * n {
        return (side, pixels.to_vec());
    }
    let small = max as usize;
    let mut out = Vec::with_capacity(small * small);
    for y in 0..small {
        let top = y * n / small;
        let bottom = ((y + 1) * n / small).max(top + 1);
        for x in 0..small {
            let left = x * n / small;
            let right = ((x + 1) * n / small).max(left + 1);
            let mut sum = 0u32;
            let mut count = 0u32;
            for row in top..bottom {
                for column in left..right {
                    if let Some(&pixel) = pixels.get(row * n + column) {
                        sum += u32::from(pixel);
                        count += 1;
                    }
                }
            }
            out.push((sum / count.max(1)) as u8);
        }
    }
    (max, out)
}

/// A grey PNG, 8 bits a pixel.
pub fn gray_png(side: u32, pixels: &[u8]) -> Result<Vec<u8>> {
    encode(side, side, png::ColorType::Grayscale, pixels)
}

/// A white-on-transparent PNG whose alpha is the grey image: how Krita takes a tip.
pub fn alpha_png(pixels: &[u8]) -> Result<Vec<u8>> {
    let side = (pixels.len() as f64).sqrt().round() as u32;
    let mut rgba = Vec::with_capacity(pixels.len() * 4);
    for &alpha in pixels {
        rgba.extend_from_slice(&[255, 255, 255, alpha]);
    }
    encode(side, side, png::ColorType::Rgba, &rgba)
}

/// An 8-bit RGBA PNG.
pub fn rgba_png(width: u32, height: u32, pixels: &[u8]) -> Result<Vec<u8>> {
    encode(width, height, png::ColorType::Rgba, pixels)
}

fn encode(width: u32, height: u32, color: png::ColorType, pixels: &[u8]) -> Result<Vec<u8>> {
    let expected = width as usize * height as usize * color.samples();
    if pixels.len() != expected {
        return Err(Error::new(format!("image is {} bytes, expected {expected} for {width}×{height}", pixels.len())));
    }
    let mut out = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut out, width, height);
        encoder.set_color(color);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header()?;
        writer.write_image_data(pixels)?;
        writer.finish()?;
    }
    Ok(out)
}

/// A 1×1 transparent PNG, for a file that must hold a thumbnail.
pub fn blank_png() -> Result<Vec<u8>> {
    rgba_png(1, 1, &[0, 0, 0, 0])
}

/// A PNG's pixels as grey (lightness times alpha), and its size.
#[cfg(test)]
pub fn decode_gray(bytes: &[u8]) -> Result<(u32, u32, Vec<u8>)> {
    let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info()?;
    let mut buffer = vec![0; reader.output_buffer_size().unwrap_or(0)];
    let info = reader.next_frame(&mut buffer)?;
    let (width, height) = (info.width, info.height);
    let (color, depth) = reader.output_color_type();
    if depth != png::BitDepth::Eight {
        return Err(Error::new("only 8-bit PNGs are read"));
    }
    buffer.truncate(info.buffer_size());
    let pixels = match color {
        png::ColorType::Grayscale => buffer,
        png::ColorType::GrayscaleAlpha => buffer.as_chunks::<2>().0.iter().map(|p| mul(p, 1)).collect(),
        png::ColorType::Rgb => buffer.as_chunks::<3>().0.iter().map(|p| grey(p)).collect(),
        png::ColorType::Rgba => buffer.as_chunks::<4>().0.iter().map(|p| mul(p, 3)).collect(),
        png::ColorType::Indexed => return Err(Error::new("indexed PNGs are not read")),
    };
    Ok((width, height, pixels))
}

#[cfg(test)]
fn grey(rgb: &[u8]) -> u8 {
    let r = u32::from(rgb.first().copied().unwrap_or(0));
    let g = u32::from(rgb.get(1).copied().unwrap_or(0));
    let b = u32::from(rgb.get(2).copied().unwrap_or(0));
    ((r * 299 + g * 587 + b * 114) / 1000) as u8
}

/// The lightness of an RGB(A) pixel times its alpha.
#[cfg(test)]
fn mul(pixel: &[u8], alpha_at: usize) -> u8 {
    let alpha = u32::from(pixel.get(alpha_at).copied().unwrap_or(255));
    (u32::from(grey(pixel)) * alpha / 255) as u8
}

/// The brush's preview as PNG: the one its file brought, else a stroke drawn here, else a blank.
pub fn thumbnail_png(brush: &Brush) -> Vec<u8> {
    if let Some(png) = &brush.thumbnail
        && png.starts_with(&[0x89, b'P', b'N', b'G'])
    {
        return png.to_vec();
    }
    if let Ok(png) = preview::stroke(brush) {
        return png;
    }
    blank_png().unwrap_or_else(|_| Vec::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_grey_pngs() {
        let pixels: Vec<u8> = (0..64 * 64).map(|i| (i % 251) as u8).collect();
        let png = gray_png(64, &pixels).expect("encode");
        let (w, h, back) = decode_gray(&png).expect("decode");
        assert_eq!((w, h), (64, 64));
        assert_eq!(back, pixels);
    }
}
