//! A JSON description of the brushes, for scripts and for checking a conversion.

use procraft_engine::brush::Brush;
use serde_json::{Map, Value, json};

use crate::{BrushSet, Result};

/// `set` as a JSON document.
pub fn write(set: &BrushSet) -> Result<Vec<u8>> {
    let brushes: Vec<Value> = set.brushes.iter().map(brush_json).collect();
    let document = json!({
        "name": set.name,
        "file": set.file_name,
        "source": set.source.label(),
        "brushCount": set.brushes.len(),
        "brushes": brushes,
    });
    Ok(serde_json::to_vec_pretty(&document)?)
}

fn brush_json(brush: &Brush) -> Value {
    let mut extra = Map::new();
    for (key, value) in &brush.extra {
        extra.insert(key.clone(), json!(value));
    }
    json!({
        "name": brush.name,
        "spacing": brush.spacing,
        "hardness": brush.hardness,
        "size": { "min": brush.min_size, "max": brush.max_size },
        "opacity": { "min": brush.min_opacity, "max": brush.max_opacity },
        "flow": brush.flow,
        "pressure": {
            "size": brush.pressure_size,
            "opacity": brush.pressure_opacity,
        },
        "shape": {
            "roundTip": brush.shape.image.is_none() && brush.shape.source.is_none(),
            "rotation": brush.shape.rotation,
            "roundness": brush.shape.roundness,
            "scatter": brush.shape.scatter,
            "count": brush.shape.count,
        },
        "grain": {
            "hasImage": brush.grain.image.is_some() || brush.grain.source.is_some(),
            "mode": format!("{:?}", brush.grain.mode),
            "scale": brush.grain.scale,
            "depth": brush.grain.depth,
        },
        "dual": brush.secondary.as_ref().map(|second| brush_json(second)),
        "extra": Value::Object(extra),
    })
}
