//! The converter's UniFFI surface, which the Android converter app calls.
//!
//! The command line tool (`tools/brushconv`) and this share the same conversion code; this only
//! wraps it in records and an error that UniFFI carries to Kotlin. Nothing here paints.

use std::path::Path;

use brushconv::fields::Field;
use brushconv::{OutputFormat, read_bytes, write_set};

uniffi::setup_scaffolding!();

/// One output format the converter writes, for the app's format list.
#[derive(Clone, Debug, uniffi::Record)]
pub struct FormatInfo {
    /// The name `convert` takes, and the file extension.
    pub name: String,
    /// What to call it in the interface.
    pub label: String,
    /// Whether a set becomes one file per brush rather than one file.
    pub one_per_brush: bool,
    /// Whether several inputs can be combined into one output.
    pub merges: bool,
}

/// One brush, as the app lists it after reading a file.
#[derive(Clone, Debug, uniffi::Record)]
pub struct BrushInfo {
    pub name: String,
    pub min_size: f64,
    pub max_size: f64,
    /// A fraction of the tip's size (0.08 is 8%).
    pub spacing: f64,
    pub min_opacity: f64,
    pub max_opacity: f64,
}

/// What a brush file held, for the app's report of the selection.
#[derive(Clone, Debug, uniffi::Record)]
pub struct Inspection {
    /// The kind of file, e.g. "ProCraft brush set".
    pub format: String,
    /// The set's own name.
    pub set_name: String,
    pub brushes: Vec<BrushInfo>,
}

/// One field of a brush, as the app's editor lists it.
#[derive(Clone, Debug, uniffi::Record)]
pub struct FieldInfo {
    /// A stable name, e.g. "size".
    pub key: String,
    /// What to call it in the interface.
    pub label: String,
    /// The brush's value for it, already written for people.
    pub value: String,
    /// Whether the file it came from carries this field.
    pub imported: bool,
    /// Whether the chosen output format writes this field.
    pub exportable: bool,
}

/// One brush's fields, for the editor.
#[derive(Clone, Debug, uniffi::Record)]
pub struct BrushFields {
    pub name: String,
    pub fields: Vec<FieldInfo>,
}

/// Why a file couldn't be read or written.
#[derive(Clone, Debug, thiserror::Error, uniffi::Error)]
pub enum ConvertError {
    #[error("{reason}")]
    Failed { reason: String },
}

impl ConvertError {
    fn failed(reason: impl Into<String>) -> ConvertError {
        ConvertError::Failed { reason: reason.into() }
    }
}

/// The formats the converter writes, in the order the app offers them.
#[uniffi::export]
pub fn formats() -> Vec<FormatInfo> {
    [
        OutputFormat::Brush,
        OutputFormat::BrushSet,
        OutputFormat::Abr,
        OutputFormat::Kpp,
        OutputFormat::Bundle,
        OutputFormat::Gbr,
        OutputFormat::Prbr,
        OutputFormat::Json,
    ]
    .into_iter()
    .map(|format| FormatInfo {
        name: format.extension().to_owned(),
        label: format.label().to_owned(),
        one_per_brush: format.one_per_brush(),
        merges: format.merges_inputs(),
    })
    .collect()
}

/// Reads a brush file, for the app to show what it holds.
#[uniffi::export]
pub fn inspect(name: String, bytes: Vec<u8>) -> Result<Inspection, ConvertError> {
    let set = read_bytes(&bytes, &name).map_err(|error| ConvertError::failed(error.to_string()))?;
    let brushes = set
        .brushes
        .iter()
        .map(|brush| BrushInfo {
            name: brush.name.clone(),
            min_size: f64::from(brush.min_size),
            max_size: f64::from(brush.max_size),
            spacing: f64::from(brush.spacing),
            min_opacity: f64::from(brush.min_opacity),
            max_opacity: f64::from(brush.max_opacity),
        })
        .collect();
    Ok(Inspection { format: set.source.label().to_owned(), set_name: set.name, brushes })
}

/// Every brush in a file with the fields the editor shows: which of them `target` keeps, and
/// which ones the file itself carried.
#[uniffi::export]
pub fn fields(name: String, bytes: Vec<u8>, target: String) -> Result<Vec<BrushFields>, ConvertError> {
    let output = OutputFormat::from_name(&target)
        .ok_or_else(|| ConvertError::failed(format!("there is no format called '{target}'")))?;
    let set = read_bytes(&bytes, &name).map_err(|error| ConvertError::failed(error.to_string()))?;
    Ok(set
        .brushes
        .iter()
        .map(|brush| BrushFields {
            name: brush.name.clone(),
            fields: brushconv::fields::of(brush, set.source, output).into_iter().map(field).collect(),
        })
        .collect())
}

fn field(field: Field) -> FieldInfo {
    FieldInfo {
        key: field.key.to_owned(),
        label: field.label.to_owned(),
        value: field.value,
        imported: field.imported,
        exportable: field.exportable,
    }
}

/// Turns a brush file into `format`, writing the files into `out_dir` (which the app owns), and
/// returns their names. One set can become several files (`.kpp`, `.gbr`, `.brush`).
#[uniffi::export]
pub fn convert(
    name: String,
    bytes: Vec<u8>,
    format: String,
    set_name: Option<String>,
    out_dir: String,
) -> Result<Vec<String>, ConvertError> {
    let output = OutputFormat::from_name(&format)
        .ok_or_else(|| ConvertError::failed(format!("there is no format called '{format}'")))?;
    let dir = Path::new(&out_dir);
    std::fs::create_dir_all(dir).map_err(|error| ConvertError::failed(format!("can't make {out_dir}: {error}")))?;
    let set = read_bytes(&bytes, &name).map_err(|error| ConvertError::failed(error.to_string()))?;
    let written = write_set(&set, output, Some(dir), set_name.as_deref())
        .map_err(|error| ConvertError::failed(error.to_string()))?;
    let names = written
        .into_iter()
        .filter_map(|path| path.file_name().map(|name| name.to_string_lossy().into_owned()))
        .collect();
    Ok(names)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inking() -> Vec<u8> {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../legacy-brushes/Inking.brushset");
        std::fs::read(path).unwrap()
    }

    #[test]
    fn lists_every_format() {
        let formats = formats();
        let names: Vec<&str> = formats.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, ["brush", "brushset", "abr", "kpp", "bundle", "gbr", "prbr", "json"]);
        assert!(formats.iter().find(|f| f.name == "kpp").unwrap().one_per_brush);
    }

    #[test]
    fn the_editor_marks_what_an_abr_export_drops() {
        let brushes = fields("Inking.brushset".to_owned(), inking(), "abr".to_owned()).expect("fields");
        assert_eq!(brushes.len(), 12);
        assert_eq!(brushes[0].name, "Round Pen");
        let size = brushes[0].fields.iter().find(|f| f.key == "size").expect("size");
        assert!(size.imported, "the ProCraft set carries a size range");
        assert!(!size.exportable, "an ABR does not");
        let tip = brushes[0].fields.iter().find(|f| f.key == "tip").expect("tip");
        assert!(tip.imported && tip.exportable);
    }

    #[test]
    fn inspects_the_inking_set() {
        let inspection = inspect("Inking.brushset".to_owned(), inking()).unwrap();
        assert_eq!(inspection.format, "ProCraft brush set");
        assert_eq!(inspection.brushes.len(), 12);
        assert_eq!(inspection.brushes.first().unwrap().name, "Round Pen");
    }

    #[test]
    fn converts_a_set_into_one_bundle() {
        let dir = std::env::temp_dir().join(format!("brushconv-ffi-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let names = convert(
            "Inking.brushset".to_owned(),
            inking(),
            "bundle".to_owned(),
            Some("Inking FFI".to_owned()),
            dir.to_string_lossy().into_owned(),
        )
        .unwrap();
        assert_eq!(names, ["Inking FFI.bundle"]);
        assert!(dir.join("Inking FFI.bundle").is_file());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_brush_per_file_format_writes_each_brush() {
        let dir = std::env::temp_dir().join(format!("brushconv-ffi-kpp-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let names =
            convert("Inking.brushset".to_owned(), inking(), "kpp".to_owned(), None, dir.to_string_lossy().into_owned())
                .unwrap();
        assert_eq!(names.len(), 12);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_unknown_format_is_an_error() {
        let error = convert(
            "Inking.brushset".to_owned(),
            inking(),
            "nope".to_owned(),
            None,
            std::env::temp_dir().to_string_lossy().into_owned(),
        )
        .unwrap_err();
        assert!(error.to_string().contains("no format called"));
    }
}
