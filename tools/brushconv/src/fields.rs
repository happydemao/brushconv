//! Which of a brush's fields a format carries: what an import brought in, and what an export can
//! take away. The app shows this beside each field so a conversion's losses are visible before it
//! happens rather than after.

use procraft_engine::brush::{Brush, PressureCurve};
use procraft_engine::curve::Curve;

use crate::read::SourceFormat;
use crate::write::OutputFormat;

/// One field of a brush, as the editor lists it.
pub struct Field {
    pub key: &'static str,
    pub label: &'static str,
    pub value: String,
    pub imported: bool,
    pub exportable: bool,
    /// The input-to-output curve behind a pressure dynamic, 0..1 each, for the editor to draw.
    pub curve: Option<Vec<(f32, f32)>>,
}

/// Every field the editor knows, in the order it shows them.
const FIELDS: [(&str, &str); 20] = [
    ("name", "Name"),
    ("tip", "Tip"),
    ("grain", "Grain"),
    ("spacing", "Spacing"),
    ("hardness", "Hardness"),
    ("size", "Size range"),
    ("opacity", "Opacity"),
    ("flow", "Flow"),
    ("pressure_size", "Pressure → size"),
    ("pressure_opacity", "Pressure → opacity"),
    ("pressure_flow", "Pressure → flow"),
    ("pressure_roundness", "Pressure → roundness"),
    ("scatter", "Scatter"),
    ("jitter", "Size jitter"),
    ("rotation", "Rotation"),
    ("roundness", "Roundness"),
    ("follow", "Follows direction"),
    ("dual", "Dual brush"),
    ("author", "Author"),
    ("thumbnail", "Thumbnail"),
];

/// Everything ProCraft's own files keep.
const ALL: [&str; 20] = [
    "name",
    "tip",
    "grain",
    "spacing",
    "hardness",
    "size",
    "opacity",
    "flow",
    "pressure_size",
    "pressure_opacity",
    "pressure_flow",
    "pressure_roundness",
    "scatter",
    "jitter",
    "rotation",
    "roundness",
    "follow",
    "dual",
    "author",
    "thumbnail",
];

/// What a format's reader sets, as far as the editor shows it. A brush file carries more than
/// this; these are the parts ProCraft turns into a `Brush`.
fn reads(source: SourceFormat) -> &'static [&'static str] {
    match source {
        SourceFormat::ProCraft | SourceFormat::ProCraftSet | SourceFormat::ProCraftLibrary => &ALL,
        SourceFormat::Abr => &["name", "tip", "spacing", "scatter", "rotation", "roundness", "dual"],
        SourceFormat::Gbr | SourceFormat::Gih | SourceFormat::Vbr => &["tip"],
        SourceFormat::Krita | SourceFormat::KritaBundle => &[
            "name",
            "tip",
            "grain",
            "spacing",
            "hardness",
            "size",
            "opacity",
            "flow",
            "pressure_size",
            "pressure_opacity",
            "pressure_flow",
            "pressure_roundness",
            "scatter",
            "jitter",
            "rotation",
            "roundness",
            "dual",
        ],
        SourceFormat::Csp | SourceFormat::CspGroup => {
            &["name", "tip", "grain", "spacing", "hardness", "opacity", "flow", "pressure_size", "dual"]
        }
        SourceFormat::Prbr => &[
            "name",
            "tip",
            "grain",
            "spacing",
            "hardness",
            "size",
            "opacity",
            "flow",
            "pressure_size",
            "pressure_flow",
            "scatter",
            "jitter",
            "rotation",
        ],
        SourceFormat::Unknown => &[],
    }
}

/// What a writer keeps of the brush.
fn writes(target: OutputFormat) -> &'static [&'static str] {
    match target {
        OutputFormat::Brush | OutputFormat::BrushSet | OutputFormat::Json => &ALL,
        OutputFormat::Abr => &["name", "tip", "spacing"],
        OutputFormat::Gbr => &["name", "tip", "spacing"],
        OutputFormat::Kpp | OutputFormat::Bundle => &[
            "name",
            "tip",
            "grain",
            "spacing",
            "hardness",
            "size",
            "opacity",
            "flow",
            "pressure_size",
            "pressure_opacity",
            "pressure_flow",
            "pressure_roundness",
            "scatter",
            "jitter",
            "rotation",
            "roundness",
            "dual",
        ],
        OutputFormat::Prbr => &[
            "name",
            "tip",
            "grain",
            "spacing",
            "hardness",
            "size",
            "opacity",
            "flow",
            "pressure_size",
            "pressure_flow",
            "scatter",
            "jitter",
            "rotation",
        ],
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
            curve: curve(brush, key),
        })
        .collect()
}

/// The curve behind a dynamic field, in the editor's 0..1 space, when the field is one.
fn curve(brush: &Brush, key: &str) -> Option<Vec<(f32, f32)>> {
    let points = |curve: &Curve| curve.points().iter().map(|point| (point.x, point.y)).collect::<Vec<_>>();
    let set = |curve: Option<&Curve>| curve.map(&points);
    match key {
        "pressure_size" if brush.pressure_size > 0.0 => Some(points(&brush.size_curve)),
        "pressure_opacity" if brush.pressure_opacity > 0.0 => Some(points(&brush.opacity_curve)),
        "pressure_flow" => {
            let driven = brush.setting("pressure_flow") > 0.0;
            set(brush.pressure_curves.get(PressureCurve::Flow)).or_else(|| driven.then(|| vec![(0.0, 0.0), (1.0, 1.0)]))
        }
        "pressure_roundness" => {
            let driven = brush.setting("pressure_roundness") > 0.0;
            set(brush.pressure_curves.get(PressureCurve::Roundness))
                .or_else(|| driven.then(|| vec![(0.0, 0.0), (1.0, 1.0)]))
        }
        _ => None,
    }
}

/// Whether the brush actually has something in `key`: a format that never carries a field, or a
/// file that left it at its default, both show as not imported.
fn present(brush: &Brush, key: &str) -> bool {
    match key {
        "name" => !brush.name.trim().is_empty(),
        "tip" => brush.shape.source.is_some() || brush.shape.image.is_some() || brush.hardness < 1.0,
        "grain" => brush.grain.source.is_some() || brush.grain.image.is_some() || brush.grain_file.is_some(),
        "spacing" => brush.spacing > 0.0,
        "hardness" => brush.hardness < 1.0,
        "size" => brush.max_size > brush.min_size,
        "opacity" => brush.max_opacity > brush.min_opacity || brush.max_opacity < 1.0,
        "flow" => brush.flow < 0.999,
        "pressure_size" => brush.pressure_size > 0.0,
        "pressure_opacity" => brush.pressure_opacity > 0.0,
        "pressure_flow" => brush.setting("pressure_flow") > 0.0 || curve(brush, key).is_some(),
        "pressure_roundness" => brush.setting("pressure_roundness") > 0.0 || curve(brush, key).is_some(),
        "scatter" => brush.setting("scatter") > 0.0,
        "jitter" => brush.setting("jitter_size") > 0.0 || brush.shape.size_jitter > 0.0,
        "rotation" => brush.shape.rotation != 0.0,
        "roundness" => brush.shape.roundness < 1.0,
        "follow" => brush.shape.follow != 0.0,
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
        "hardness" => format!("{}%", percent(brush.hardness)),
        "size" => format!("{} – {}", size(brush.min_size), size(brush.max_size)),
        "opacity" => format!("{} – {}%", percent(brush.min_opacity), percent(brush.max_opacity)),
        "flow" => format!("{}%", percent(brush.flow)),
        "pressure_size" | "pressure_opacity" | "pressure_flow" | "pressure_roundness" => match curve(brush, key) {
            Some(points) => {
                let first = points.first().map_or(0.0, |point| point.1);
                let last = points.last().map_or(1.0, |point| point.1);
                format!("{} points · {}% → {}%", points.len(), percent(first), percent(last))
            }
            None => effect(amount(brush, key)),
        },
        "scatter" => format!("{}%", percent(brush.setting("scatter"))),
        "jitter" => format!("{}%", percent(brush.setting("jitter_size").max(brush.shape.size_jitter))),
        "rotation" => format!("{}°", (f64::from(brush.shape.rotation).to_degrees()).round()),
        "roundness" => format!("{}%", percent(brush.shape.roundness)),
        "follow" => {
            if brush.shape.follow == 0.0 {
                "no".to_owned()
            } else {
                format!("{:+}%", percent(brush.shape.follow))
            }
        }
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

/// How much a dynamic drives its field, as the brush holds it.
fn amount(brush: &Brush, key: &str) -> f32 {
    match key {
        "pressure_size" => brush.pressure_size,
        "pressure_opacity" => brush.pressure_opacity,
        "pressure_flow" => brush.setting("pressure_flow"),
        "pressure_roundness" => brush.setting("pressure_roundness"),
        _ => 0.0,
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
        assert_eq!(fields.len(), 20);
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

    #[test]
    fn a_pressure_dynamic_comes_with_the_curve_to_draw() {
        let mut brush = Brush::round_pen();
        brush.pressure_size = 1.0;
        let fields = of(&brush, SourceFormat::ProCraftSet, OutputFormat::Kpp);
        let size = fields.iter().find(|f| f.key == "pressure_size").expect("size");
        let points = size.curve.as_ref().expect("a curve");
        assert!(points.len() >= 2);
        assert!(points.iter().all(|(x, y)| (0.0..=1.0).contains(x) && (0.0..=1.0).contains(y)));
        let rotation = fields.iter().find(|f| f.key == "rotation").expect("rotation");
        assert!(rotation.curve.is_none());
    }
}
