//! Infinite Painter brushes: a `.prbr` is a zip holding `properties.json`, the brush's settings in
//! sections (its head, its stroke, its dynamics' profiles and its texture), and `preview`, a PNG of
//! a sample stroke. Its head and textures are ones that come with the app, named and not held in
//! the file, unless it names a custom one, which it holds as a picture.

use serde_json::Value;

use crate::zip::ZipReader;

/// A brush as an Infinite Painter `.prbr` has it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PrbrBrush {
    pub name: String,
    /// The gap between the head's stamps, in head widths.
    pub spacing: Option<f32>,
    /// The brush's size, in pixels.
    pub size: Option<f32>,
    pub size_maximum: Option<f32>,
    pub opacity: Option<f32>,
    pub flow: Option<f32>,
    /// The head's own flow, times the blend's.
    pub head_flow: Option<f32>,
    /// How soft the head's edge is: 0 is a hard edge.
    pub softness: Option<f32>,
    pub angle: Option<f32>,
    /// Whether the pressure rules the size, and its profile (pressure and size, in pairs).
    pub pressure_size: bool,
    pub size_profile: Vec<f32>,
    /// Whether the pressure rules the flow, and its profile.
    pub pressure_flow: bool,
    pub flow_profile: Vec<f32>,
    pub scatter: Option<f32>,
    pub size_jitter: Option<f32>,
    /// The name of the head and the texture the app has, if the brush names them.
    pub head: Option<String>,
    pub texture: Option<String>,
    pub texture_scale: Option<f32>,
    /// A head or a texture the brush holds itself (a custom one), as a PNG.
    pub custom_head_png: Option<Vec<u8>>,
    pub custom_texture_png: Option<Vec<u8>>,
    /// The sample stroke the app shows, as a PNG.
    pub preview_png: Option<Vec<u8>>,
}

/// The brush an Infinite Painter `.prbr` holds, if it holds one.
pub fn read(bytes: &[u8]) -> Option<PrbrBrush> {
    let zip = ZipReader::new(bytes)?;
    let json: Value = serde_json::from_slice(&zip.read("properties.json")?).ok()?;
    let name = json.get("display-name").and_then(Value::as_str)?.trim();
    if name.is_empty() {
        return None;
    }
    let head = json.get("head-properties");
    let stroke = json.get("stroke-properties");
    let blend = json.get("blend-properties");
    let source = json.get("source-properties");
    let texture = json.get("texture-properties");
    let dynamics = json.get("dynamics-properties");
    let head_flow = number(head, "flow");
    let blend_flow = number(blend, "flow");
    Some(PrbrBrush {
        name: name.to_owned(),
        spacing: number(head, "spacing").filter(|s| *s > 0.0),
        size: number(stroke, "paint-size"),
        size_maximum: number(stroke, "size-maximum"),
        opacity: number(stroke, "paint-opacity"),
        flow: blend_flow.or(head_flow),
        head_flow,
        softness: number(head, "softness"),
        angle: number(head, "angle"),
        pressure_size: effect(dynamics, "pressure - effects", "size"),
        size_profile: numbers(dynamics, "pressure profile - size"),
        pressure_flow: effect(dynamics, "pressure - effects", "flow"),
        flow_profile: numbers(dynamics, "pressure profile - flow"),
        scatter: number(json.get("jitter-properties"), "scatter"),
        size_jitter: number(json.get("jitter-properties"), "size"),
        head: string_of(source, "head"),
        texture: string_of(source, "stroke texture"),
        texture_scale: number(texture, "scale"),
        custom_head_png: resource(&zip, string_of(source, "custom-head").as_deref()),
        custom_texture_png: resource(&zip, string_of(source, "custom-stroke texture").as_deref()),
        preview_png: zip.read("preview").filter(|p| p.starts_with(b"\x89PNG")),
    })
}

/// A section's number, if it has one.
fn number(section: Option<&Value>, key: &str) -> Option<f32> {
    section.and_then(|s| s.get(key)).and_then(Value::as_f64).map(|n| n as f32).filter(|n| n.is_finite())
}

/// An effect's switch: the app writes it flat ("pressure - effects size"), an older export of ours
/// nested it under the effect's name; both read.
fn effect(section: Option<&Value>, name: &str, key: &str) -> bool {
    flag(section, &format!("{name} {key}")) || flag(section.and_then(|s| s.get(name)), key)
}

/// A section's switch, if it has one.
fn flag(section: Option<&Value>, key: &str) -> bool {
    section.and_then(|s| s.get(key)).and_then(Value::as_bool).unwrap_or(false)
}

/// A section's text, if it has one and it isn't empty.
fn string_of(section: Option<&Value>, key: &str) -> Option<String> {
    section.and_then(|s| s.get(key)).and_then(Value::as_str).map(str::trim).filter(|s| !s.is_empty()).map(str::to_owned)
}

/// A section's numbers, in their order (the profiles are pressure and value, in pairs).
fn numbers(section: Option<&Value>, key: &str) -> Vec<f32> {
    section
        .and_then(|s| s.get(key))
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(Value::as_f64).map(|n| n as f32).collect())
        .unwrap_or_default()
}

/// The picture the brush holds for a resource it names by a path: by that path, its file's name, or
/// the end of its path.
fn resource(zip: &ZipReader, path: Option<&str>) -> Option<Vec<u8>> {
    let path = path?;
    let file = path.rsplit(['/', '\\']).next()?;
    if let Some(bytes) = zip.read(path) {
        return Some(bytes);
    }
    if let Some(bytes) = zip.read(file) {
        return Some(bytes);
    }
    let name = zip.names().find(|n| n.ends_with(file))?.to_owned();
    zip.read(&name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::zip::ZipWriter;

    fn sample() -> Vec<u8> {
        let properties = r#"{
            "display-name": "Gesture Vine",
            "blend-properties": { "flow": 1.0 },
            "head-properties": { "angle": 3.1415927, "flow": 0.9082, "softness": 0.25, "spacing": 0.012976 },
            "stroke-properties": { "paint-size": 18.96, "paint-opacity": 1.0 },
            "source-properties": { "custom-head": "", "custom-stroke texture": "", "head": "head_chalk_faded",
                "stroke texture": "texture_4" },
            "texture-properties": { "scale": 1.0 },
            "jitter-properties": { "scatter": 0.5, "size": 0.25 },
            "dynamics-properties": {
                "pressure - effects size": true,
                "pressure - effects flow": true,
                "pressure profile - size": [0.0, 1.0, 0.375, 0.35477942, 1.0, 0.0],
                "pressure profile - flow": [0.0, 0.98897, 1.0, 0.0]
            }
        }"#;
        let mut zip = ZipWriter::new();
        zip.add("properties.json", properties.as_bytes());
        zip.add("preview", b"\x89PNG preview");
        zip.finish()
    }

    #[test]
    fn a_brush_reads_its_settings() {
        let brush = read(&sample()).unwrap();
        assert_eq!(brush.name, "Gesture Vine");
        assert_eq!(brush.spacing, Some(0.012976));
        assert_eq!(brush.size, Some(18.96));
        assert_eq!(brush.softness, Some(0.25));
        assert_eq!(brush.flow, Some(1.0));
        assert_eq!(brush.head_flow, Some(0.9082));
        assert!(brush.pressure_size && brush.pressure_flow);
        assert_eq!(brush.size_profile, vec![0.0, 1.0, 0.375, 0.35477942, 1.0, 0.0]);
        assert_eq!(brush.scatter, Some(0.5));
        assert_eq!(brush.head.as_deref(), Some("head_chalk_faded"));
        assert_eq!(brush.texture.as_deref(), Some("texture_4"));
        assert_eq!(brush.preview_png.as_deref(), Some(b"\x89PNG preview".as_slice()));
    }

    #[test]
    fn an_effect_nested_under_its_name_reads_too() {
        let properties = r#"{
            "display-name": "Nested",
            "dynamics-properties": { "pressure - effects": { "size": true, "flow": false } }
        }"#;
        let mut zip = ZipWriter::new();
        zip.add("properties.json", properties.as_bytes());
        let brush = read(&zip.finish()).unwrap();
        assert!(brush.pressure_size);
        assert!(!brush.pressure_flow);
    }

    #[test]
    fn a_file_that_is_not_a_brush_reads_none() {
        assert!(read(b"not a zip").is_none());
        assert!(read(&ZipWriter::new().finish()).is_none());
    }
}
