//! The real ProCraft sets in `legacy-brushes/`, through every writer and back.

#![recursion_limit = "256"]

use std::path::{Path, PathBuf};

use brushconv::{OutputFormat, read_path, write_set};

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../legacy-brushes/Inking.brushset")
}

/// A folder nothing else uses, removed when `Drop` runs.
struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Scratch {
        let dir = std::env::temp_dir().join(format!("brushconv-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::create_dir_all(&dir);
        Scratch(dir)
    }

    fn path(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn the_inking_set_reads() {
    let set = read_path(&fixture()).expect("read Inking.brushset");
    assert_eq!(set.brushes.len(), 12);
    assert_eq!(set.brushes[0].name, "Round Pen");
    assert_eq!(set.name, "Inking");
}

#[test]
fn a_brushset_round_trips_through_every_format_that_holds_a_set() {
    let set = read_path(&fixture()).expect("read");
    let brush = set.brushes[0].clone();
    let scratch = Scratch::new("set");
    for (format, name) in [
        (OutputFormat::BrushSet, "Inking.brushset"),
        (OutputFormat::Abr, "Inking.abr"),
        (OutputFormat::Bundle, "Inking.bundle"),
    ] {
        let out = scratch.path(name);
        write_set(&set, format, Some(&out), None).expect("write");
        assert!(out.is_file(), "{name} was written");
        let back = read_path(&out).expect("read back");
        assert_eq!(back.brushes.len(), 12, "{name} kept every brush");
        assert_eq!(back.brushes[0].name, brush.name, "{name} kept the names");
    }

    let json = scratch.path("Inking.json");
    write_set(&set, OutputFormat::Json, Some(&json), None).expect("write json");
    let text = std::fs::read_to_string(&json).expect("read json");
    let parsed: serde_json::Value = serde_json::from_str(&text).expect("parse json");
    assert_eq!(parsed["brushCount"], 12);
}

#[test]
fn one_brush_round_trips_through_the_one_brush_formats() {
    let set = read_path(&fixture()).expect("read");
    let brush = set.brushes[0].clone();
    let scratch = Scratch::new("one");
    for (format, name) in [
        (OutputFormat::Brush, "Round Pen.brush"),
        (OutputFormat::Kpp, "Round Pen.kpp"),
        (OutputFormat::Gbr, "Round Pen.gbr"),
        (OutputFormat::Prbr, "Round Pen.prbr"),
        (OutputFormat::Sai2, "Round Pen.saitdat"),
        (OutputFormat::Pcbrushes, "Round Pen.pcbrushes"),
    ] {
        let single = brushconv::BrushSet {
            name: brush.name.clone(),
            brushes: vec![brush.clone()],
            source: set.source,
            file_name: name.to_owned(),
        };
        let out = scratch.path(name);
        write_set(&single, format, Some(&out), None).expect("write");
        let back = read_path(&out).expect("read back");
        assert_eq!(back.brushes.len(), 1, "{name} holds one brush");
        assert_eq!(back.brushes[0].name, brush.name, "{name} kept the name");
    }
}

#[test]
fn every_legacy_set_converts() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../legacy-brushes");
    let scratch = Scratch::new("legacy");
    let mut count = 0;
    for entry in std::fs::read_dir(dir).expect("legacy-brushes").flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|e| e == "brushset") {
            let set = read_path(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
            let name = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
            write_set(&set, OutputFormat::Abr, Some(&scratch.path(&format!("{name}.abr"))), None)
                .unwrap_or_else(|e| panic!("{name} to abr: {e}"));
            count += 1;
        }
    }
    assert!(count > 20, "there are {count} legacy sets");
}

/// A SAI2 custom tool's readings, and its conversion into the formats that carry them.
#[test]
fn a_sai2_tool_becomes_a_brush() {
    let tool = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/Water soft.saitdat");
    let set = read_path(&tool).expect("read the SAI2 tool");
    assert_eq!(set.brushes.len(), 1);
    let brush = &set.brushes[0];
    assert_eq!(brush.name, "Water soft");
    assert_eq!(set.source, brushconv::SourceFormat::Sai2);
    // 200 px across, never below 30% of that
    assert!((brush.max_size - 200.0).abs() < 0.5, "max {}", brush.max_size);
    assert!((brush.min_size - 60.0).abs() < 0.5, "min {}", brush.min_size);
    // density 80%, least density 20%
    assert!((brush.max_opacity - 0.8).abs() < 1e-6);
    assert!((brush.min_opacity - 0.2).abs() < 1e-6);
    // wavno 1 of 4
    assert!((brush.hardness - 0.25).abs() < 1e-6);
    // water 40%, mixing 50%, colour stretch 25%
    assert!((brush.setting("wet_dilution") - 0.4).abs() < 1e-6);
    assert!((brush.setting("wet_pull") - 0.5).abs() < 1e-6);
    assert!((brush.setting("wet_charge") - 0.25).abs() < 1e-6);
    assert!(brush.mixes_wet());
    assert!(brush.pressure_size > 0.0 && brush.pressure_opacity > 0.0);

    let scratch = Scratch::new("sai");
    let out = scratch.path("Water soft.kpp");
    let set = read_path(&tool).expect("read again");
    write_set(&set, brushconv::OutputFormat::Kpp, Some(&out), None).expect("write kpp");
    assert!(out.is_file());
    let back = read_path(&out).expect("read back");
    assert!((back.brushes[0].max_size - 200.0).abs() < 0.5);
}

#[test]
fn a_photocraft_group_round_trips() {
    let set = read_path(&fixture()).expect("read");
    let scratch = Scratch::new("pcbrushes");
    let out = scratch.path("Inking.pcbrushes");
    let written = write_set(&set, OutputFormat::Pcbrushes, Some(&out), None).expect("write");
    assert_eq!(written.len(), 1, "a group holds every brush in one file");
    let back = read_path(&out).expect("read back");
    assert_eq!(back.source, brushconv::SourceFormat::Pcbrushes);
    assert_eq!(back.brushes.len(), 12);
    for (before, after) in set.brushes.iter().zip(&back.brushes) {
        assert_eq!(before.name, after.name);
        assert!(
            (before.max_size - after.max_size).abs() < 0.5,
            "{}: {} vs {}",
            before.name,
            before.max_size,
            after.max_size
        );
    }
}

/// A `.prbr` built by hand, its settings set to values a brush of the app's would have.
fn a_prbr(properties: &serde_json::Value) -> Vec<u8> {
    let mut zip = procraft_engine::zip::ZipWriter::new();
    zip.add("properties.json", properties.to_string().as_bytes());
    zip.finish()
}

#[test]
fn a_brush_keeps_the_settings_it_came_with() {
    let settings = serde_json::json!({
        "display-name": "Keeps",
        "parent": 2,
        "blend-properties": {"flow": 0.67, "mix-in": 0.2, "mix-in dilution": 0.18, "mix-wet": false, "smudge-amount": 0.79, "strength": 0.2},
        "head-properties": {"angle": 0.5, "depth": 0.2, "flow": 0.86, "rotation": 1, "screen-aligned": true, "softness": 0.3, "spacing": 0.05, "structure": 0.2, "style": 1, "use-trajectory": true},
        "stroke-properties": {"adhesion": 0.27, "blend-opacity": 1, "blend-size": 40, "blendmode": 4, "blendmode-intensity": 0.52, "erase-opacity": 1, "erase-size": 40, "flow-profile": [0, 0, 0.3, 0, 1, 1], "glaze-strokes": 0.31, "has-flow-profile": true, "has-size-profile": true, "lazy-stroke": 0.33, "paint-opacity": 0.8, "paint-size": 40, "size-maximum": 1.7, "size-profile": [0, 1, 0.5, 0, 1, 1], "wet-edges": 0.145},
        "source-properties": {"custom-head": "", "custom-stroke texture": "", "head": "", "head - color": false, "head - colorize": 1, "head - conversion format": 1, "head - profile": [0, 1, 1, 0], "stroke texture": "", "stroke texture - color": false, "stroke texture - colorize": 1, "stroke texture - profile": [0, 1, 1, 0]},
        "dynamics-properties": {"pressure - effects size": true, "pressure - effects flow": true, "pressure - effects dilution": true, "pressure - effects head depth": true, "pressure - effects scatter": true, "pressure - effects texture": true, "pressure profile - size": [0, 1, 0.33, 0.34, 1, 0], "pressure profile - flow": [0, 1, 0.33, 0.34, 1, 0], "pressure profile - dilution": [0.2, 0, 0.4, 0.1, 0.6, 0.9, 0.8, 1], "pressure profile - head depth": [0, 1, 0.58, 0.31, 1, 0], "pressure profile - scatter": [0, 1, 0.5, 0.9, 1, 0.6], "pressure profile - texture": [0, 0.5, 0.5, 0.3, 1, 0], "tilt - effects size": true, "tilt - effects flow": true, "tilt - effects dilution": true, "tilt - effects head depth": true, "tilt - effects scatter": true, "tilt - effects texture": true, "tilt - gradation": 0.8, "tilt - offset": 0.8, "tilt profile - size": [0, 1, 0.5, 1, 1, 0.75], "tilt profile - flow": [0, 0, 0.5, 0, 1, 0.75], "tilt profile - dilution": [0.2, 1, 0.4, 0.9, 0.6, 0.1, 0.8, 0], "tilt profile - head depth": [0, 1, 1, 0], "tilt profile - scatter": [0, 1, 0.5, 1, 1, 0.4], "tilt profile - texture": [0, 0, 0.5, 0.2, 1, 0.6], "velocity - effects size": true, "velocity - effects flow": true, "velocity - effects dilution": true, "velocity - effects head depth": true, "velocity - effects scatter": true, "velocity - effects texture": true, "velocity profile - size": [0, 0, 0.5, 0.7, 1, 1], "velocity profile - flow": [0, 0, 0.5, 0.7, 1, 1], "velocity profile - dilution": [0, 0, 1, 1], "velocity profile - head depth": [0, 1, 1, 0], "velocity profile - scatter": [0, 0, 0.5, 0.1, 1, 0.4], "velocity profile - texture": [0, 1, 0.5, 0.5, 1, 0]},
        "jitter-properties": {"angle": 0.33, "color-brightness": 0.35, "color-hue": 0.31, "color-saturation": 0.37, "color-start brightness": 0.2, "color-start hue": 0.23, "color-start saturation": 0.31, "flow": 0.34, "scatter": 0.4, "size": 0.25, "start-angle": 0.18, "texture-angle": 0, "texture-position": 0, "texture-scale": 0, "texture-start-position": 0},
        "texture-properties": {"angle": 0, "invert": false, "pressure": 0.6, "scale": 1.4, "scale-size": true, "softness": 0, "stretch": 1, "structure": 0, "style": 1},
    });
    let source = a_prbr(&settings);
    let set = brushconv::read_bytes(&source, "Keeps.prbr").expect("read");
    let scratch = Scratch::new("prbr-settings");
    let out = scratch.path("Keeps.prbr");
    write_set(&set, OutputFormat::Prbr, Some(&out), None).expect("write");
    let back: serde_json::Value = serde_json::from_slice(
        &procraft_engine::zip::ZipReader::new(&std::fs::read(&out).expect("file"))
            .expect("zip")
            .read("properties.json")
            .expect("settings"),
    )
    .expect("json");
    // Every number and switch comes back as it went in; only the name and the pictures differ.
    for (section, values) in settings.as_object().expect("object") {
        if section == "display-name" || section == "source-properties" {
            continue;
        }
        let Some(values) = values.as_object() else { continue };
        let written = &back[section.as_str()];
        for (key, value) in values {
            match value {
                serde_json::Value::Number(number) => {
                    let got = written[key].as_f64().unwrap_or(f64::NAN);
                    assert!(
                        (got - number.as_f64().expect("number")).abs() < 1e-5,
                        "{section}/{key}: {got} for {number}"
                    );
                }
                serde_json::Value::Bool(flag) => {
                    assert_eq!(written[key].as_bool(), Some(*flag), "{section}/{key}");
                }
                serde_json::Value::Array(items) => {
                    let got = written[key].as_array().expect("array");
                    assert_eq!(got.len(), items.len(), "{section}/{key}");
                    for (i, item) in items.iter().enumerate() {
                        let want = item.as_f64().expect("point");
                        let got = got[i].as_f64().unwrap_or(f64::NAN);
                        assert!((got - want).abs() < 1e-5, "{section}/{key}[{i}]: {got} for {want}");
                    }
                }
                _ => {}
            }
        }
    }
}
