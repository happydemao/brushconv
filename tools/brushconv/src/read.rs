//! Reading any brush file ProCraft reads into ProCraft's brush model.

use std::path::Path;

use procraft_core::ReadBrushes;

use crate::{BrushSet, Error, Result};

/// What kind of brush file an input was.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceFormat {
    /// ProCraft's own `.brush`.
    ProCraft,
    /// ProCraft's own `.brushset`.
    ProCraftSet,
    /// ProCraft's `.brushlibrary`.
    ProCraftLibrary,
    /// Photoshop's `.abr`.
    Abr,
    /// GIMP's `.gbr`.
    Gbr,
    /// GIMP's `.gih` brush pipe.
    Gih,
    /// GIMP's `.vbr` generated brush.
    Vbr,
    /// Krita's `.kpp` preset.
    Krita,
    /// Krita's `.bundle` resource bundle.
    KritaBundle,
    /// Clip Studio's `.sut` sub tool.
    Csp,
    /// Clip Studio's `.sutg` sub tool group.
    CspGroup,
    /// Infinite Painter's `.prbr`.
    Prbr,
    /// PaintTool SAI2's custom tool, `.saitdat`.
    Sai2,
    /// PhotoCraft's `.pcbrushes` brush group.
    Pcbrushes,
    /// Recognised by its contents but not its name.
    Unknown,
}

impl SourceFormat {
    pub fn label(self) -> &'static str {
        match self {
            SourceFormat::ProCraft => "ProCraft brush",
            SourceFormat::ProCraftSet => "ProCraft brush set",
            SourceFormat::ProCraftLibrary => "ProCraft brush library",
            SourceFormat::Abr => "Photoshop ABR",
            SourceFormat::Gbr => "GIMP brush",
            SourceFormat::Gih => "GIMP brush pipe",
            SourceFormat::Vbr => "GIMP generated brush",
            SourceFormat::Krita => "Krita preset",
            SourceFormat::KritaBundle => "Krita bundle",
            SourceFormat::Csp => "Clip Studio sub tool",
            SourceFormat::CspGroup => "Clip Studio sub tool group",
            SourceFormat::Prbr => "Infinite Painter brush",
            SourceFormat::Sai2 => "PaintTool SAI2 tool",
            SourceFormat::Pcbrushes => "PhotoCraft brush group",
            SourceFormat::Unknown => "brush file",
        }
    }

    /// Whether the format holds more than one brush in a single file.
    pub fn is_container(self) -> bool {
        matches!(
            self,
            SourceFormat::ProCraftSet
                | SourceFormat::ProCraftLibrary
                | SourceFormat::Abr
                | SourceFormat::KritaBundle
                | SourceFormat::CspGroup
                | SourceFormat::Pcbrushes
        )
    }
}

/// What kind of file `bytes` called `file_name` is, by its name, then by its contents.
pub fn detect_format(bytes: &[u8], file_name: &str) -> SourceFormat {
    let ext = file_name.rsplit_once('.').map(|(_, ext)| ext.to_ascii_lowercase()).unwrap_or_default();
    let by_name = match ext.as_str() {
        "brush" => Some(SourceFormat::ProCraft),
        "brushset" => Some(SourceFormat::ProCraftSet),
        "brushlibrary" => Some(SourceFormat::ProCraftLibrary),
        "abr" => Some(SourceFormat::Abr),
        "gbr" => Some(SourceFormat::Gbr),
        "gih" => Some(SourceFormat::Gih),
        "vbr" => Some(SourceFormat::Vbr),
        "kpp" => Some(SourceFormat::Krita),
        "bundle" => Some(SourceFormat::KritaBundle),
        "sut" => Some(SourceFormat::Csp),
        "sutg" => Some(SourceFormat::CspGroup),
        "prbr" => Some(SourceFormat::Prbr),
        "saitdat" => Some(SourceFormat::Sai2),
        "pcbrushes" => Some(SourceFormat::Pcbrushes),
        _ => None,
    };
    if let Some(format) = by_name {
        return format;
    }
    sniff(bytes)
}

/// The kind of file `bytes` is by its contents alone, best effort.
fn sniff(bytes: &[u8]) -> SourceFormat {
    if bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]) {
        return SourceFormat::Krita;
    }
    if let Some(zip) = procraft_engine::zip::ZipReader::new(bytes) {
        let names: Vec<&str> = zip.names().collect();
        if names.contains(&"brushset.plist") {
            return SourceFormat::ProCraftSet;
        }
        if names.iter().any(|n| *n == "Brush.archive" || n.ends_with("/Brush.archive")) {
            return SourceFormat::ProCraft;
        }
        if names.contains(&"properties.json") && names.contains(&"preview") {
            return SourceFormat::Prbr;
        }
        if names.contains(&"META-INF/manifest.xml") || names.iter().any(|n| n.starts_with("paintoppresets/")) {
            return SourceFormat::KritaBundle;
        }
        if names.iter().any(|n| n.starts_with("brushes/")) {
            return SourceFormat::ProCraftLibrary;
        }
    }
    if bytes.starts_with(b"SQLite format 3\0") {
        return SourceFormat::Csp;
    }
    if looks_like_sai2(bytes) {
        return SourceFormat::Sai2;
    }
    if looks_like_pcbrushes(bytes) {
        return SourceFormat::Pcbrushes;
    }
    if looks_like_gbr(bytes) {
        return SourceFormat::Gbr;
    }
    if looks_like_abr(bytes) {
        return SourceFormat::Abr;
    }
    SourceFormat::Unknown
}

/// A PhotoCraft group is JSON naming its own format near the start.
fn looks_like_pcbrushes(bytes: &[u8]) -> bool {
    let head = bytes.get(..2048).unwrap_or(bytes);
    String::from_utf8_lossy(head).contains("photocraft-brush-group")
}

/// A SAI2 custom tool is text with a `tidstr=` header and a `--EOF--` end.
fn looks_like_sai2(bytes: &[u8]) -> bool {
    let head = bytes.get(..4096).unwrap_or(bytes);
    let text = String::from_utf8_lossy(head);
    text.contains("tidstr=") && (text.contains("--EOF--") || text.contains("tfmode="))
}

fn u32_at(bytes: &[u8], at: usize) -> Option<u32> {
    bytes.get(at..at + 4).and_then(|b| b.try_into().ok()).map(u32::from_be_bytes)
}

fn looks_like_gbr(bytes: &[u8]) -> bool {
    let (Some(header), Some(version)) = (u32_at(bytes, 0), u32_at(bytes, 4)) else { return false };
    let (Some(width), Some(height)) = (u32_at(bytes, 8), u32_at(bytes, 12)) else { return false };
    header >= 20
        && header <= bytes.len() as u32
        && matches!(version, 1..=3)
        && width > 0
        && height > 0
        && width <= 5000
        && height <= 5000
}

fn looks_like_abr(bytes: &[u8]) -> bool {
    let Some(version) = bytes.get(0..2).and_then(|b| b.try_into().ok()).map(u16::from_be_bytes) else {
        return false;
    };
    let Some(count) = bytes.get(2..4).and_then(|b| b.try_into().ok()).map(u16::from_be_bytes) else {
        return false;
    };
    matches!(version, 1 | 2) && count > 0 && count < 4096
}

/// Reads the brushes in `bytes`, called `file_name`, from whichever app's file it is.
pub fn read_bytes(bytes: &[u8], file_name: &str) -> Result<BrushSet> {
    if bytes.is_empty() {
        return Err(Error::new(format!("{file_name} is empty")));
    }
    let source = detect_format(bytes, file_name);
    let stem = Path::new(file_name)
        .file_stem()
        .map(|s| s.to_string_lossy().trim().to_owned())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "Brushes".to_owned());
    let read = procraft_core::read_any_brushes(bytes, Some(file_name.to_owned())).ok_or_else(|| {
        Error::new(format!(
            "could not read {file_name} as a brush file ({}). Supported: .brush, .brushset, \
             .brushlibrary, .abr, .kpp, .bundle, .gbr, .gih, .vbr, .sut, .sutg, .prbr, .saitdat, \
             .pcbrushes",
            source.label()
        ))
    })?;
    let (name, brushes) = match read {
        ReadBrushes::Set { name, brushes, .. } => (name, brushes),
        ReadBrushes::One(brush) => {
            let name = if brush.name.trim().is_empty() { stem.clone() } else { brush.name.clone() };
            (name, vec![*brush])
        }
        ReadBrushes::Library { name, sets, .. } => {
            let brushes: Vec<procraft_engine::brush::Brush> =
                sets.into_iter().flat_map(|(_, _, brushes)| brushes).collect();
            (name, brushes)
        }
        ReadBrushes::Text(_) => {
            return Err(Error::new(format!(
                "{file_name} is a ProCraft text brush file from an older version; open it in \
                 ProCraft once to save it as a .brush, then convert that"
            )));
        }
    };
    if brushes.is_empty() {
        return Err(Error::new(format!("{file_name} holds no brushes ProCraft can read")));
    }
    let name = if name.trim().is_empty() { stem } else { name.trim().to_owned() };
    Ok(BrushSet { name, brushes, source, file_name: file_name.to_owned() })
}

/// Reads the brushes in the file at `path`.
pub fn read_path(path: &Path) -> Result<BrushSet> {
    let bytes = std::fs::read(path)?;
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    read_bytes(&bytes, &name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sniffs_the_formats_by_name() {
        assert_eq!(detect_format(b"", "a.abr"), SourceFormat::Abr);
        assert_eq!(detect_format(b"", "a.ABR"), SourceFormat::Abr);
        assert_eq!(detect_format(b"", "a.sutg"), SourceFormat::CspGroup);
        assert_eq!(detect_format(b"", "a.kpp"), SourceFormat::Krita);
        assert_eq!(detect_format(b"", "infinite painter.prbr"), SourceFormat::Prbr);
        assert_eq!(detect_format(b"", "Brush.SAITDAT"), SourceFormat::Sai2);
    }

    #[test]
    fn sniffs_a_sai2_tool_by_its_text() {
        let tool = b"tidstr=S:brush\nname=U:Soft\n.\nmodstr=I:1\n.\n--EOF--\n";
        assert_eq!(detect_format(tool, "no-name"), SourceFormat::Sai2);
        assert_eq!(detect_format(b"just a text file", "no-name"), SourceFormat::Unknown);
    }

    #[test]
    fn sniffs_an_infinite_painter_brush_by_its_zip() {
        let mut zip = procraft_engine::zip::ZipWriter::new();
        zip.add("properties.json", b"{}");
        zip.add("preview", b"png");
        assert_eq!(detect_format(&zip.finish(), "no-extension"), SourceFormat::Prbr);
    }
}
