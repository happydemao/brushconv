//! Infinite Painter's `.prbr`: a zip of `properties.json` (the brush's settings) and `preview` (a
//! sample stroke), and the head and texture pictures when the brush has ones of its own.
//!
//! The app's own heads and textures come with it, named rather than held. A brush ProCraft draws
//! with a picture of its own goes in as a custom head: an entry named `head` (and `texture` for a
//! grain), named by the BLAKE2b-512 digest of its bytes in `custom-head` / `custom-stroke texture`,
//! with `head` / `stroke texture` left empty. A plain round brush keeps the app's own round head and
//! carries its falloff in `softness`. The settings are the app's own sections with the parts
//! ProCraft has no control for at the app's defaults, and every setting the brush brought with it
//! (its `ip/…` extras) is put back over them, so a `.prbr` written from a `.prbr` keeps all of it.

use procraft_engine::brush::{Brush, PressureCurve};
use procraft_engine::curve::Curve;
use procraft_engine::zip::{Ready, ZipWriter};
use serde_json::{Value, json};

use crate::Result;
use crate::images;
use crate::preview;

/// The pixels Infinite Painter's `paint-size` counts a point as, the calibration the converters
/// that read the format use.
const PIXELS_PER_SIZE: f32 = 3.74;

/// The head and texture entries: the app holds its custom pictures under these plain names and
/// names them by their digests, not by a path.
const HEAD: &str = "head";
const TEXTURE: &str = "texture";

/// The preview the app draws into the brush list.
const PREVIEW: (u32, u32) = (512, 128);

/// One brush as a `.prbr`.
pub fn write(brush: &Brush) -> Result<Vec<u8>> {
    let head = if has_picture(brush) { Some(head_png(brush)?) } else { None };
    let texture = texture_png(brush)?;
    let source = source_section(head.as_deref(), &texture);
    let properties = json!({
        "display-name": brush.name.clone(),
        "parent": 356,
        "blend-properties": blend_section(brush),
        "head-properties": head_section(brush),
        "stroke-properties": stroke_section(brush),
        "source-properties": source,
        "dynamics-properties": dynamics_section(brush),
        "jitter-properties": jitter_section(brush),
        "texture-properties": texture_section(),
    });
    let mut properties = properties;
    apply_extras(&mut properties, &brush.extra);
    whole_numbers(&mut properties);
    // The app's own exports store every entry, so doing the same keeps the archive as close to
    // what it writes as the settings are.
    let mut zip = ZipWriter::new();
    zip.add_ready("preview", Ready::of(&preview::stroke_sized(brush, PREVIEW.0, PREVIEW.1)?, false));
    if let Some(head) = &head {
        zip.add_ready(HEAD, Ready::of(head, false));
    }
    if let Some(texture) = &texture {
        zip.add_ready(TEXTURE, Ready::of(texture, false));
    }
    zip.add_ready("properties.json", Ready::of(properties.to_string().as_bytes(), false));
    Ok(zip.finish())
}

/// Whether ProCraft paints this brush with a picture of its own, which the app has to be handed,
/// rather than a round tip it draws from the hardness.
fn has_picture(brush: &Brush) -> bool {
    brush.shape.source.is_some() || brush.shape.image.is_some()
}

/// The brush's head: ink on white, which the app paints where it is dark, as its own files hold
/// their heads.
fn head_png(brush: &Brush) -> Result<Vec<u8>> {
    let (side, pixels) = images::tip_gray(brush);
    let (_, pixels) = images::fit_gray(side, &pixels, 512);
    images::ink_png(&pixels)
}

/// The brush's grain as the app's texture: a grey picture, whose bright parts paint.
fn texture_png(brush: &Brush) -> Result<Option<Vec<u8>>> {
    let Some((side, pixels)) = images::grain_gray(brush) else {
        return Ok(None);
    };
    let (side, pixels) = images::fit_gray(side, &pixels, 512);
    images::gray_png(side, &pixels).map(Some)
}

fn blend_section(brush: &Brush) -> Value {
    json!({
        "flow": round(brush.flow.clamp(0.0, 1.0)),
        "mix-in": 0.0,
        "mix-in dilution": 0.0,
        "mix-wet": false,
        "smudge-amount": 1.0,
        "strength": 0.2,
    })
}

fn head_section(brush: &Brush) -> Value {
    json!({
        "angle": round(brush.shape.rotation.rem_euclid(std::f32::consts::TAU)),
        "depth": 0.0,
        "flow": round(brush.flow.clamp(0.0, 1.0)),
        "rotation": 0.0,
        "screen-aligned": false,
        "softness": round((1.0 - brush.hardness).clamp(0.0, 1.0)),
        "spacing": round(brush.spacing.clamp(0.001, 2.0)),
        "structure": 0.0,
        "style": 0.0,
        "use-trajectory": true,
    })
}

fn stroke_section(brush: &Brush) -> Value {
    let least = brush.min_opacity.clamp(0.0, 1.0);
    let most = brush.max_opacity.clamp(0.0, 1.0).max(least);
    let size = pixels(brush);
    json!({
        "adhesion": 0.0,
        "blend-opacity": 1.0,
        "blend-size": 40.0,
        "blendmode": 0.0,
        "blendmode-intensity": 1.0,
        "erase-opacity": 1.0,
        "erase-size": 40.0,
        "flow-profile": [0.0, 0.0, 0.3, 0.0, 1.0, 1.0],
        "glaze-strokes": 0.0,
        "has-flow-profile": false,
        "has-size-profile": false,
        "lazy-stroke": 0.0,
        "paint-opacity": round(most),
        "paint-size": round(size),
        "size-maximum": 1.0,
        "size-profile": [0.0, 1.0, 0.5, 0.0, 1.0, 1.0],
        "wet-edges": 0.0,
    })
}

fn source_section(head: Option<&[u8]>, texture: &Option<Vec<u8>>) -> Value {
    let texture_hash = texture.as_ref().map(|bytes| crate::blake2b::hex(bytes)).unwrap_or_default();
    let head_hash = head.map(crate::blake2b::hex).unwrap_or_default();
    json!({
        "custom-head": head_hash,
        "custom-stroke texture": texture_hash,
        "head": "",
        // The head is ink on white, so the app paints where it is dark.
        "head - color": false,
        "head - colorize": 1.0,
        "head - conversion format": 1.0,
        "head - profile": [0.0, 1.0, 1.0, 0.0],
        "stroke texture": "",
        "stroke texture - color": false,
        "stroke texture - colorize": 1.0,
        "stroke texture - profile": [0.0, 1.0, 1.0, 0.0],
    })
}

fn dynamics_section(brush: &Brush) -> Value {
    let size_effect = brush.pressure_size > 0.0;
    let flow_curve = brush.pressure_curves.get(PressureCurve::Flow);
    let flow_effect = flow_curve.is_some() || brush.setting("pressure_flow") > 0.0;
    let size_profile = if size_effect { profile(&brush.size_curve) } else { vec![0.0, 1.0, 1.0, 1.0] };
    let flow_profile = flow_curve.map(profile).unwrap_or_else(|| vec![0.0, 1.0, 1.0, 1.0]);
    json!({
        "pressure - effects size": size_effect,
        "pressure - effects flow": flow_effect,
        "pressure - effects dilution": false,
        "pressure - effects head depth": false,
        "pressure - effects scatter": false,
        "pressure - effects texture": false,
        "pressure profile - size": size_profile,
        "pressure profile - flow": flow_profile,
        "pressure profile - dilution": [0.2, 0.0, 0.4, 0.1, 0.6, 0.9, 0.8, 1.0],
        "pressure profile - head depth": [0.0, 1.0, 1.0, 0.0],
        "pressure profile - scatter": [0.0, 1.0, 0.5, 0.9, 1.0, 0.6],
        "pressure profile - texture": [0.0, 0.5, 0.5, 0.3, 1.0, 0.0],
        "tilt - effects size": false,
        "tilt - effects flow": false,
        "tilt - effects dilution": false,
        "tilt - effects head depth": false,
        "tilt - effects scatter": false,
        "tilt - effects texture": false,
        "tilt - gradation": 0.8,
        "tilt - offset": 0.8,
        "tilt profile - size": [0.0, 1.0, 0.5, 1.0, 1.0, 0.0],
        "tilt profile - flow": [0.0, 0.0, 0.5, 0.0, 1.0, 0.75],
        "tilt profile - dilution": [0.2, 1.0, 0.4, 0.9, 0.6, 0.1, 0.8, 0.0],
        "tilt profile - head depth": [0.0, 1.0, 1.0, 0.0],
        "tilt profile - scatter": [0.0, 1.0, 0.5, 1.0, 1.0, 0.4],
        "tilt profile - texture": [0.0, 0.0, 0.5, 0.2, 1.0, 0.6],
        "velocity - effects size": false,
        "velocity - effects flow": false,
        "velocity - effects dilution": false,
        "velocity - effects head depth": false,
        "velocity - effects scatter": false,
        "velocity - effects texture": false,
        "velocity profile - size": [0.0, 0.0, 0.5, 0.5, 1.0, 1.0],
        "velocity profile - flow": [0.0, 0.0, 0.5, 0.5, 1.0, 1.0],
        "velocity profile - dilution": [0.0, 0.0, 1.0, 1.0],
        "velocity profile - head depth": [0.0, 1.0, 1.0, 0.0],
        "velocity profile - scatter": [0.0, 0.0, 0.5, 0.2, 1.0, 0.4],
        "velocity profile - texture": [0.0, 1.0, 0.5, 0.5, 1.0, 0.0],
    })
}

fn jitter_section(brush: &Brush) -> Value {
    json!({
        "angle": 0.0,
        "color-brightness": 0.0,
        "color-hue": 0.0,
        "color-saturation": 0.0,
        "color-start brightness": 0.0,
        "color-start hue": 0.0,
        "color-start saturation": 0.0,
        "flow": 0.0,
        "scatter": round(brush.setting("scatter").clamp(0.0, 1.0)),
        "size": round(brush.setting("jitter_size").clamp(0.0, 1.0)),
        "start-angle": 0.0,
        "texture-angle": 0.0,
        "texture-position": 0.0,
        "texture-scale": 0.0,
        "texture-start-position": 0.0,
    })
}

fn texture_section() -> Value {
    json!({
        "angle": 0.0,
        "invert": false,
        "pressure": 0.0,
        "scale": 1.0,
        "scale-size": false,
        "softness": 0.0,
        "stretch": 1.0,
        "structure": 0.0,
        "style": 1.0,
    })
}

/// The brush's size in the app's points: its own pixels over the app's pixels a point.
fn pixels(brush: &Brush) -> f32 {
    (brush.max_size / PIXELS_PER_SIZE).clamp(0.5, 5000.0)
}

/// Puts the settings the brush brought back over the app's defaults: `ip/<section>/<setting>`
/// numbers and switches, `…#<index>` for the points of a profile.
fn apply_extras(properties: &mut Value, extra: &std::collections::BTreeMap<String, f32>) {
    for (key, number) in extra {
        let Some(path) = key.strip_prefix("ip/") else { continue };
        let (path, switch) = match path.strip_suffix("#b") {
            Some(path) => (path, true),
            None => (path, false),
        };
        if switch {
            let parts: Vec<&str> = path.split('/').collect();
            put_switch(properties, &parts, *number > 0.5);
            continue;
        }
        let (path, index) = match path.rsplit_once('#') {
            Some((path, index)) => match index.parse::<usize>() {
                Ok(index) => (path, Some(index)),
                Err(_) => continue,
            },
            None => (path, None),
        };
        let parts: Vec<&str> = path.split('/').collect();
        put(properties, &parts, index, *number);
    }
}

/// Sets one switch, making the sections on the way to it as it goes.
fn put_switch(properties: &mut Value, parts: &[&str], on: bool) {
    let Some((last, parents)) = parts.split_last() else { return };
    let Some(map) = properties.as_object_mut() else { return };
    if parents.is_empty() {
        map.insert((*last).to_owned(), Value::Bool(on));
    } else {
        let (Some((first, _)), Some(rest)) = (parents.split_first(), parts.get(1..)) else { return };
        let section = map.entry((*first).to_owned()).or_insert_with(|| Value::Object(Default::default()));
        put_switch(section, rest, on);
    }
}

/// Sets one setting, making the sections on the way to it as it goes.
fn put(properties: &mut Value, parts: &[&str], index: Option<usize>, number: f32) {
    let Some((last, parents)) = parts.split_last() else { return };
    let Some(map) = properties.as_object_mut() else { return };
    if parents.is_empty() {
        let leaf = match index {
            Some(index) => {
                let entry = map.entry((*last).to_owned()).or_insert_with(|| Value::Array(Vec::new()));
                let Some(items) = entry.as_array_mut() else { return };
                items.resize(index + 1, Value::from(0));
                match items.get_mut(index) {
                    Some(leaf) => leaf,
                    None => return,
                }
            }
            None => map.entry((*last).to_owned()).or_insert(Value::Null),
        };
        *leaf = Value::from(f64::from(number));
    } else {
        let (Some((first, _)), Some(rest)) = (parents.split_first(), parts.get(1..)) else { return };
        let section = map.entry((*first).to_owned()).or_insert_with(|| Value::Object(Default::default()));
        put(section, rest, index, number);
    }
}

/// Writes whole numbers as whole numbers: the app's own files have `"style": 1`, not `1.0`, and a
/// strict reader can tell the difference.
fn whole_numbers(value: &mut Value) {
    match value {
        Value::Number(number) => {
            if let Some(float) = number.as_f64()
                && float.fract() == 0.0
                && float.abs() < 9_000_000_000_000_000.0
            {
                *number = serde_json::Number::from(float as i64);
            }
        }
        Value::Array(items) => items.iter_mut().for_each(whole_numbers),
        Value::Object(map) => map.values_mut().for_each(whole_numbers),
        _ => {}
    }
}

/// A curve as the app's flat profile: pressure and value, in pairs.
fn profile(curve: &Curve) -> Vec<f32> {
    curve.points().iter().flat_map(|p| [round32(p.x), round32(p.y)]).collect()
}

/// A number as JSON writes it without noise well past what a brush control needs.
fn round(value: f32) -> f64 {
    (f64::from(value) * 1e6).round() / 1e6
}

/// [`round`], staying in `f32` for a profile's own numbers.
fn round32(value: f32) -> f32 {
    (value * 1e6).round() / 1e6
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_a_readable_infinite_painter_brush() {
        let mut brush = Brush::round_pen();
        brush.name = "Soft Round".to_owned();
        brush.pressure_size = 1.0;
        brush.hardness = 0.4;
        let bytes = write(&brush).expect("write");
        let read = procraft_engine::prbr::read(&bytes).expect("read back");
        assert_eq!(read.name, "Soft Round");
        assert_eq!(read.softness, Some(0.6));
        assert!(read.pressure_size);
        // A round brush keeps the app's own head, so no picture goes in.
        assert!(read.custom_head_png.is_none());
        assert_eq!(read.head, None);
        assert!(read.preview_png.is_some());
    }

    #[test]
    fn the_app_names_its_custom_head_by_the_picture_s_digest() {
        let mut brush = Brush::round_pen();
        brush.shape.image = Some(0);
        let bytes = write(&brush).expect("write");
        let zip = procraft_engine::zip::ZipReader::new(&bytes).expect("zip");
        assert!(zip.names().any(|name| name == "head"));
        let json: serde_json::Value =
            serde_json::from_slice(&zip.read("properties.json").expect("settings")).expect("json");
        let source = &json["source-properties"];
        assert_eq!(source["head"], "");
        assert_eq!(source["stroke texture"], "");
        assert_eq!(source["head - conversion format"], 1);
        let head = zip.read("head").expect("head");
        assert_eq!(source["custom-head"].as_str(), Some(crate::blake2b::hex(&head).as_str()));
        assert_eq!(source["custom-head"].as_str().map(str::len), Some(128));
    }

    #[test]
    fn whole_settings_are_written_as_whole_numbers() {
        let brush = Brush::round_pen();
        let bytes = write(&brush).expect("write");
        let text = String::from_utf8_lossy(&bytes);
        assert!(text.contains("\"paint-opacity\":1"), "got {text:.600}");
        assert!(!text.contains("\"paint-opacity\":1.0"));
        assert!(text.contains("\"spacing\":0.08") || text.contains("\"spacing\":0.1"));
    }
}
