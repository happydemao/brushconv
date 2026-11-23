//! PhotoCraft brush groups: a `.pcbrushes` file is JSON holding one library group's presets,
//! the brush settings in PhotoCraft's own field names, and each bitmap they paint with replaced
//! by a reference to a `tips/<hash>.pctip` file, plus that tip's small preview picture inline.
//!
//! PhotoCraft keeps these files in its `Presets` folder, one per group, beside an `index.json`
//! that records their order; a group file dropped there is read on the next start. Its `.abr`
//! importer is the only brush-file importer it offers, so reading this format is the way a
//! PhotoCraft library leaves the app.

use serde_json::Value;

use crate::abr::AbrTip;
use crate::base64;

/// The brushes of one PhotoCraft group.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PcGroup {
    /// The group's name (PhotoCraft's "folder" for the presets).
    pub name: String,
    pub brushes: Vec<PcBrush>,
}

/// A brush as a PhotoCraft preset holds it: the tip shape's settings and the dynamics that map
/// onto ours. PhotoCraft's other sections (texture, dual brush, colour dynamics, transfer, pose,
/// smoothing, mixer) are not read.
#[derive(Clone, Debug, PartialEq)]
pub struct PcBrush {
    pub name: String,
    /// Diameter in pixels.
    pub size: f32,
    /// 0 (soft) ..1 (hard).
    pub hardness: f32,
    /// The gap between dabs, as a fraction of the diameter.
    pub spacing: f32,
    /// Maximum coverage for the stroke.
    pub opacity: f32,
    /// Per-dab coverage.
    pub flow: f32,
    /// Degrees, counter-clockwise.
    pub angle: f32,
    /// 0..1 (1 = circle).
    pub roundness: f32,
    pub flip_x: bool,
    pub flip_y: bool,
    pub pressure_size: bool,
    pub pressure_opacity: bool,
    /// The tip's bitmap: the inline preview of a sampled tip. A computed (round) tip has none.
    pub tip: Option<AbrTip>,
}

impl Default for PcBrush {
    fn default() -> Self {
        Self {
            name: String::new(),
            size: 20.0,
            hardness: 1.0,
            spacing: 0.1,
            opacity: 1.0,
            flow: 1.0,
            angle: 0.0,
            roundness: 1.0,
            flip_x: false,
            flip_y: false,
            pressure_size: false,
            pressure_opacity: false,
            tip: None,
        }
    }
}

/// The format name PhotoCraft writes in every group file.
pub const GROUP_FORMAT: &str = "photocraft-brush-group";
/// The only group-file version PhotoCraft reads.
pub const GROUP_VERSION: u32 = 1;

/// The group a PhotoCraft `.pcbrushes` file holds, if it holds one.
pub fn read(bytes: &[u8]) -> Option<PcGroup> {
    let json: Value = serde_json::from_slice(bytes).ok()?;
    if json.get("format").and_then(Value::as_str)? != GROUP_FORMAT {
        return None;
    }
    if json.get("version").and_then(Value::as_u64).unwrap_or(0) != u64::from(GROUP_VERSION) {
        return None;
    }
    let name = json
        .get("group")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("PhotoCraft")
        .to_owned();
    let presets = json.get("presets")?.as_array()?;
    let mut brushes = Vec::new();
    for preset in presets {
        let Some(name) = preset.get("name").and_then(Value::as_str).map(str::trim).filter(|s| !s.is_empty()) else {
            continue;
        };
        let brush = preset.get("brush");
        let mut pc = PcBrush { name: name.to_owned(), ..PcBrush::default() };
        pc.size = number(brush, "size").filter(|v| *v > 0.0).unwrap_or(pc.size);
        pc.hardness = number(brush, "hardness").map_or(pc.hardness, |v| v.clamp(0.0, 1.0));
        pc.spacing = number(brush, "spacing").filter(|v| *v > 0.0).map_or(pc.spacing, |v| v.clamp(0.0, 2.0));
        pc.opacity = number(brush, "opacity").map_or(pc.opacity, |v| v.clamp(0.0, 1.0));
        pc.flow = number(brush, "flow").map_or(pc.flow, |v| v.clamp(0.0, 1.0));
        pc.angle = number(brush, "angle").unwrap_or(0.0);
        pc.roundness = number(brush, "roundness").map_or(1.0, |v| v.clamp(0.0, 1.0));
        pc.flip_x = flag(brush, "flipX");
        pc.flip_y = flag(brush, "flipY");
        pc.pressure_size = flag(brush, "pressureSize");
        pc.pressure_opacity = flag(brush, "pressureOpacity");
        pc.tip = brush.and_then(|b| b.get("tip")).and_then(tip_of);
        brushes.push(pc);
    }
    (!brushes.is_empty()).then_some(PcGroup { name, brushes })
}

/// A number in the settings, if the field has one.
fn number(brush: Option<&Value>, key: &str) -> Option<f32> {
    brush.and_then(|b| b.get(key)).and_then(Value::as_f64).map(|v| v as f32).filter(|v| v.is_finite())
}

/// A settings switch.
fn flag(brush: Option<&Value>, key: &str) -> bool {
    brush.and_then(|b| b.get(key)).and_then(Value::as_bool).unwrap_or(false)
}

/// The bitmap a tip paints with: a sampled tip holds it, a stored one holds its small preview
/// until the preset store decodes the full `tips/<hash>.pctip` file.
fn tip_of(tip: &Value) -> Option<AbrTip> {
    if let Some(inner) = tip.get("sampled") {
        return tile(inner);
    }
    tip.get("stored").and_then(|s| s.get("preview")).and_then(tile)
}

/// A grayscale bitmap as PhotoCraft writes it: its size and its 16-bit samples in base64.
fn tile(value: &Value) -> Option<AbrTip> {
    let width = u32::try_from(value.get("width")?.as_u64()?).ok()?;
    let height = u32::try_from(value.get("height")?.as_u64()?).ok()?;
    let count = usize::try_from(u64::from(width).checked_mul(u64::from(height))?).ok()?;
    let bytes = base64::decode(value.get("data")?.as_str()?)?;
    if count == 0 || bytes.len() != count * 2 {
        return None;
    }
    let pixels = bytes.as_chunks::<2>().0.iter().map(|pair| (u16::from_le_bytes(*pair) / 257) as u8).collect();
    Some(AbrTip { name: String::new(), spacing: None, width, height, pixels })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn group(presets: Value) -> Vec<u8> {
        serde_json::to_vec(&serde_json::json!({
            "format": "photocraft-brush-group",
            "version": 1,
            "group": "Inking",
            "presets": presets,
            "tips": {},
        }))
        .unwrap_or_default()
    }

    #[test]
    fn reads_a_round_preset() {
        let bytes = group(serde_json::json!([{
            "name": "Round Pen",
            "brush": {
                "size": 12.0, "hardness": 0.8, "spacing": 0.08, "opacity": 0.5,
                "flow": 0.9, "angle": 30.0, "roundness": 0.6, "flipY": true,
                "pressureSize": true, "tip": "round"
            }
        }]));
        let g = read(&bytes).expect("a group");
        assert_eq!(g.name, "Inking");
        let b = g.brushes.first().expect("a brush");
        assert_eq!(b.name, "Round Pen");
        assert!((b.size - 12.0).abs() < 1e-6 && (b.hardness - 0.8).abs() < 1e-6);
        assert!((b.spacing - 0.08).abs() < 1e-6 && (b.opacity - 0.5).abs() < 1e-6);
        assert!((b.angle - 30.0).abs() < 1e-6 && (b.roundness - 0.6).abs() < 1e-6);
        assert!(b.flip_y && b.pressure_size && !b.flip_x);
        assert!(b.tip.is_none());
    }

    #[test]
    fn reads_a_stored_tip_from_its_preview() {
        // A 2×2 preview: black, white, half, white (16-bit samples little-endian).
        let samples: Vec<u8> = [0u16, 65535, 32768, 65535].iter().flat_map(|v| v.to_le_bytes()).collect();
        let data = base64::encode(&samples);
        let bytes = group(serde_json::json!([{
            "name": "Chalk",
            "brush": {"size": 40.0, "tip": {"stored": {"key": "abc", "width": 2, "height": 2,
                "preview": {"width": 2, "height": 2, "data": data}}}}
        }]));
        let g = read(&bytes).expect("a group");
        let tip = g.brushes.first().and_then(|b| b.tip.as_ref()).expect("a tip");
        assert_eq!((tip.width, tip.height), (2, 2));
        assert_eq!(tip.pixels, vec![0, 255, 127, 255]);
    }

    #[test]
    fn something_else_is_not_a_group() {
        assert!(read(b"{}").is_none());
        assert!(read(b"{\"format\":\"other\",\"version\":1,\"presets\":[]}").is_none());
    }
}
