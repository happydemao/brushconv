//! PhotoCraft brush groups: a `.pcbrushes` file is JSON holding one library group's presets in
//! PhotoCraft's own field names. The app loads every group file it finds in its `Presets` folder
//! (macOS `~/Library/Application Support/Photocraft/Presets`) beside an `index.json` that only
//! sets their order, so writing one there adds the group on the next start.
//!
//! A group file keeps sampled tips in a `tips/<hash>.pctip` folder beside it, which a single-file
//! conversion cannot carry, so an exported preset names a computed tip built from its size,
//! hardness, angle and roundness. Converting to `.abr` is the way to carry a sampled bitmap.

use procraft_engine::brush::Brush;
use serde_json::{Value, json};

use crate::Result;

/// PhotoCraft's format name for a group file.
const FORMAT: &str = "photocraft-brush-group";

/// The group's presets as PhotoCraft writes them.
pub fn write(name: &str, brushes: &[Brush]) -> Result<Vec<u8>> {
    let presets: Vec<Value> = brushes.iter().map(preset).collect();
    let group = json!({
        "format": FORMAT,
        "version": 1,
        "group": name,
        "presets": presets,
        "tips": {},
    });
    Ok(serde_json::to_vec(&group)?)
}

/// One preset: its name and the settings PhotoCraft's brush model takes from ours.
fn preset(brush: &Brush) -> Value {
    json!({
        "name": brush.name,
        "brush": settings(brush),
        "tips": {},
    })
}

fn settings(brush: &Brush) -> Value {
    json!({
        "size": number(brush.max_size),
        "hardness": number(brush.hardness),
        "spacing": number(brush.spacing),
        "spacingEnabled": true,
        "opacity": number(brush.max_opacity),
        "flow": number(brush.flow),
        "pressureSize": brush.pressure_size > 0.0,
        "pressureOpacity": brush.pressure_opacity > 0.0,
        // PhotoCraft turns a tip counter-clockwise, ours clockwise
        "angle": number((-brush.shape.rotation).to_degrees().rem_euclid(360.0)),
        "roundness": number(brush.shape.roundness),
        "flipX": brush.setting("flip_x") > 0.0,
        "flipY": brush.setting("flip_y") > 0.0,
        "tip": "round",
        "erase": false,
        "behind": false,
        "noise": false,
        "wetEdges": false,
        "buildUp": false,
    })
}

/// A number JSON keeps shortly: whole values as integers, others with six decimals at most.
fn number(value: f32) -> Value {
    let value = f64::from(if value.is_finite() { value } else { 0.0 });
    let rounded = (value * 1_000_000.0).round() / 1_000_000.0;
    if rounded.fract() == 0.0 && rounded.abs() < 9.0e15 { Value::from(rounded as i64) } else { Value::from(rounded) }
}
