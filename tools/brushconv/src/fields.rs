//! Which of a brush's fields a format carries: what an import brought in, and what an export can
//! take away. The app shows this beside each field so a conversion's losses are visible before it
//! happens rather than after.

use procraft_engine::brush::Brush;

use crate::read::SourceFormat;
use crate::write::OutputFormat;

/// One field of a brush, as the editor lists it.
pub struct Field {
    pub key: &'static str,
    pub label: &'static str,
    pub value: String,
    pub imported: bool,
    pub exportable: bool,
}

/// Every field the editor knows, in the order it shows them.
const FIELDS: [(&str, &str); 16] = [
    ("name", "Name"),
    ("tip", "Tip"),
    ("grain", "Grain"),
    ("spacing", "Spacing"),
    ("size", "Size range"),
    ("opacity", "Opacity"),
    ("flow", "Flow"),
    ("pressure_size", "Pressure → size"),
    ("pressure_opacity", "Pressure → opacity"),
    ("pressure_flow", "Pressure → flow"),
    ("scatter", "Scatter"),
    ("jitter", "Size jitter"),
    ("rotation", "Rotation"),
    ("dual", "Dual brush"),
    ("author", "Author"),
    ("thumbnail", "Thumbnail"),
];

/// Everything ProCraft's own files keep.
const ALL: [&str; 16] = [
    "name",
    "tip",
    "grain",
    "spacing",
    "size",
    "opacity",
    "flow",
    "pressure_size",
    "pressure_opacity",
    "pressure_flow",
    "scatter",
    "jitter",
    "rotation",
    "dual",
    "author",
    "thumbnail",
];

/// What a format's reader sets, as far as the editor shows it. A brush file carries more than
/// this; these are the parts ProCraft turns into a `Brush`.
fn reads(format: SourceFormat) -> &'static [&'static str] {
    use SourceFormat::*;
    match format {
        ProCraft | ProCraftSet | ProCraftLibrary => &ALL,
        Abr => &["name", "tip", "spacing", "scatter", "rotation", "dual"],
        Gih | Vbr | Gbr => &["tip"],
        Krita | KritaBundle => {
            &["name", "tip", "grain", "spacing", "opacity", "flow", "pressure_size", "pressure_opacity"]
        }
        Csp | CspGroup => &["name", "tip", "grain", "spacing", "opacity", "flow", "pressure_size", "dual"],
        Prbr => &[
            "name",
            "tip",
            "grain",
            "spacing",
            "size",
            "opacity",
            "flow",
            "pressure_size",
            "pressure_flow",
            "scatter",
            "jitter",
            "rotation",
        ],
        Unknown => &[],
    }
}

/// What a format's writer sets.
fn writes(format: OutputFormat) -> &'static [&'static str] {
    use OutputFormat::*;
    match format {
        Brush | BrushSet => &ALL,
        Abr => &["name", "tip", "spacing"],
        Kpp | Bundle => &["name", "tip", "grain", "spacing", "opacity", "flow", "pressure_size", "pressure_flow"],
        Gbr => &["name", "tip", "spacing"],
        Prbr => &[
            "name",
            "tip",
            "grain",
            "spacing",
            "size",
            "opacity",
            "flow",
            "pressure_size",
            "pressure_flow",
            "scatter",
            "jitter",
            "rotation",
        ],
        Json => &ALL,
    }
}

/// `brush`'s fields, marked with what reading `source` could have brought in and what writing
/// `target` keeps.
pub fn of(brush: &Brush, source: SourceFormat, target: OutputFormat) -> Vec<Field> {
    let readable = reads(source);
    let writable = writes(target);
    FIELDS
        .iter()
        .map(|&(key, label)| Field {
            key,
            label,
            value: value(brush, key),
            imported: readable.contains(&key) && present(brush, key),
            exportable: writable.contains(&key),
        })
        .collect()
}

/// Whether the brush actually has something in `key`: a format that never carries a field, or a
/// file that left it at its default, both show as not imported.
fn present(brush: &Brush, key: &str) -> bool {
    match key {
        "name" => !brush.name.trim().is_empty(),
        "tip" => brush.shape.source.is_some() || brush.shape.image.is_some() || brush.hardness < 1.0,
        "grain" => brush.grain.source.is_some() || brush.grain.image.is_some() || brush.grain_file.is_some(),
        "spacing" => brush.spacing > 0.0,
        "size" => brush.max_size > brush.min_size,
        "opacity" => brush.max_opacity > brush.min_opacity || brush.max_opacity < 1.0,
        "flow" => brush.flow < 0.999,
        "pressure_size" => brush.pressure_size > 0.0,
        "pressure_opacity" => brush.pressure_opacity > 0.0,
        "pressure_flow" => brush.setting("pressure_flow") > 0.0,
        "scatter" => brush.setting("scatter") > 0.0,
        "jitter" => brush.setting("jitter_size") > 0.0 || brush.shape.size_jitter > 0.0,
        "rotation" => brush.shape.rotation != 0.0,
        "dual" => brush.secondary.is_some(),
        "author" => brush.author.as_ref().is_some_and(|a| !a.name.trim().is_empty()),
        "thumbnail" => brush.thumbnail.is_some(),
        _ => false,
    }
}

/// A field's value, as the editor shows it.
fn value(brush: &Brush, key: &str) -> String {
    match key {
        "name" => brush.name.clone(),
        "tip" => {
            let kind = if brush.shape.source.is_some() {
                "picture".to_owned()
            } else {
                format!("stock {}", brush.shape.image.unwrap_or(0))
            };
            format!("{kind}, hardness {}%", percent(brush.hardness))
        }
        "grain" => {
            let has = brush.grain.source.is_some() || brush.grain.image.is_some() || brush.grain_file.is_some();
            if has { "yes".to_owned() } else { "no".to_owned() }
        }
        "spacing" => format!("{}%", percent(brush.spacing)),
        "size" => format!("{} – {}", size(brush.min_size), size(brush.max_size)),
        "opacity" => format!("{} – {}%", percent(brush.min_opacity), percent(brush.max_opacity)),
        "flow" => format!("{}%", percent(brush.flow)),
        "pressure_size" => effect(brush.pressure_size),
        "pressure_opacity" => effect(brush.pressure_opacity),
        "pressure_flow" => effect(brush.setting("pressure_flow")),
        "scatter" => format!("{}%", percent(brush.setting("scatter"))),
        "jitter" => format!("{}%", percent(brush.setting("jitter_size").max(brush.shape.size_jitter))),
        "rotation" => format!("{}°", (f64::from(brush.shape.rotation).to_degrees()).round()),
        "dual" => {
            if brush.secondary.is_some() {
                "yes".to_owned()
            } else {
                "no".to_owned()
            }
        }
        "author" => brush
            .author
            .as_ref()
            .map(|a| a.name.trim().to_owned())
            .filter(|n| !n.is_empty())
            .unwrap_or_else(|| "—".to_owned()),
        "thumbnail" => {
            if brush.thumbnail.is_some() {
                "yes".to_owned()
            } else {
                "no".to_owned()
            }
        }
        _ => String::new(),
    }
}

fn effect(amount: f32) -> String {
    if amount > 0.0 { "yes".to_owned() } else { "no".to_owned() }
}

fn percent(share: f32) -> i64 {
    (f64::from(share) * 100.0).round() as i64
}

/// A size in the brush file's units.
fn size(value: f32) -> i64 {
    f64::from(value).round() as i64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::read::SourceFormat;
    use crate::write::OutputFormat;

    #[test]
    fn a_procraft_brush_imports_and_exports_everything() {
        let brush = Brush::round_pen();
        let fields = of(&brush, SourceFormat::ProCraftSet, OutputFormat::BrushSet);
        assert_eq!(fields.len(), 16);
        assert!(fields.iter().all(|f| f.exportable));
    }

    #[test]
    fn an_abr_has_no_size_range_to_export() {
        let mut brush = Brush::round_pen();
        brush.name = "Round".to_owned();
        let fields = of(&brush, SourceFormat::Abr, OutputFormat::Abr);
        let size = fields.iter().find(|f| f.key == "size").expect("size");
        assert!(!size.imported && !size.exportable);
        let tip = fields.iter().find(|f| f.key == "tip").expect("tip");
        assert!(tip.imported && tip.exportable);
    }

    #[test]
    fn a_prbr_keeps_pressure_but_not_opacity_pressure() {
        let brush = Brush::round_pen();
        let fields = of(&brush, SourceFormat::Prbr, OutputFormat::Prbr);
        let flow = fields.iter().find(|f| f.key == "pressure_flow").expect("flow");
        assert!(flow.exportable);
        let opacity = fields.iter().find(|f| f.key == "pressure_opacity").expect("opacity");
        assert!(!opacity.imported && !opacity.exportable);
    }
}
