//! The real ProCraft sets in `legacy-brushes/`, through every writer and back.

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
