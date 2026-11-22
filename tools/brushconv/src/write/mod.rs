//! Writers for each format, and where their files go.

mod abr;
mod brush;
mod gbr;
mod json;
mod krita;

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use procraft_engine::brush::Brush;

use crate::{BrushSet, Error, Result};

/// The formats the converter writes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutputFormat {
    /// ProCraft's own `.brush`, one per brush.
    Brush,
    /// ProCraft's own `.brushset`, all of them in one file.
    BrushSet,
    /// Photoshop's `.abr`, all of them in one file.
    Abr,
    /// Krita's `.kpp`, one per brush.
    Kpp,
    /// Krita's `.bundle`, all of them in one file.
    Bundle,
    /// GIMP's `.gbr`, one per brush.
    Gbr,
    /// A JSON description of every brush.
    Json,
}

impl OutputFormat {
    /// The format called `name` (as `--to` takes it), if there is one.
    pub fn from_name(name: &str) -> Option<OutputFormat> {
        match name.trim().to_ascii_lowercase().as_str() {
            "brush" | "procreate" | "procreate-brush" => Some(OutputFormat::Brush),
            "brushset" | "procreate-set" | "set" => Some(OutputFormat::BrushSet),
            "abr" | "photoshop" | "ps" => Some(OutputFormat::Abr),
            "kpp" | "krita" | "krita-preset" => Some(OutputFormat::Kpp),
            "bundle" | "krita-bundle" => Some(OutputFormat::Bundle),
            "gbr" | "gimp" | "gimp-brush" => Some(OutputFormat::Gbr),
            "json" | "meta" | "metadata" => Some(OutputFormat::Json),
            _ => None,
        }
    }

    /// The format a file ending in `ext` is.
    pub fn from_extension(ext: &str) -> Option<OutputFormat> {
        match ext.trim_start_matches('.').to_ascii_lowercase().as_str() {
            "brush" => Some(OutputFormat::Brush),
            "brushset" => Some(OutputFormat::BrushSet),
            "abr" => Some(OutputFormat::Abr),
            "kpp" => Some(OutputFormat::Kpp),
            "bundle" => Some(OutputFormat::Bundle),
            "gbr" => Some(OutputFormat::Gbr),
            "json" => Some(OutputFormat::Json),
            _ => None,
        }
    }

    pub fn extension(self) -> &'static str {
        match self {
            OutputFormat::Brush => "brush",
            OutputFormat::BrushSet => "brushset",
            OutputFormat::Abr => "abr",
            OutputFormat::Kpp => "kpp",
            OutputFormat::Bundle => "bundle",
            OutputFormat::Gbr => "gbr",
            OutputFormat::Json => "json",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            OutputFormat::Brush => "ProCraft brush",
            OutputFormat::BrushSet => "ProCraft brush set",
            OutputFormat::Abr => "Photoshop ABR",
            OutputFormat::Kpp => "Krita preset",
            OutputFormat::Bundle => "Krita bundle",
            OutputFormat::Gbr => "GIMP brush",
            OutputFormat::Json => "JSON metadata",
        }
    }

    /// Whether several inputs' brushes can be combined into one output file.
    pub fn merges_inputs(self) -> bool {
        matches!(self, OutputFormat::BrushSet | OutputFormat::Abr | OutputFormat::Bundle | OutputFormat::Json)
    }

    /// Whether a file holds one brush (so a set needs a file each).
    pub fn one_per_brush(self) -> bool {
        matches!(self, OutputFormat::Brush | OutputFormat::Kpp | OutputFormat::Gbr)
    }

    /// Whether it is a plain description rather than a brush file.
    pub fn is_json(self) -> bool {
        self == OutputFormat::Json
    }
}

/// The names of the files written for `set` in `format`, into `out` (a file for a one-file
/// format, a folder for a set of one-brush files; a folder that doesn't exist without an ending,
/// else a file).
pub fn write_set(set: &BrushSet, format: OutputFormat, out: Option<&Path>, name: Option<&str>) -> Result<Vec<PathBuf>> {
    if set.brushes.is_empty() {
        return Err(Error::new("no brushes to write"));
    }
    let title = name.map(str::to_owned).filter(|n| !n.trim().is_empty()).unwrap_or_else(|| set.name.clone());
    let title = safe_stem(&title, "Brushes");
    let one_each = format.one_per_brush() && set.brushes.len() > 1;
    if one_each {
        let dir = match out {
            Some(path) if path.is_dir() || !has_extension(path) => path.to_path_buf(),
            Some(path) => {
                return Err(Error::new(format!(
                    "{} holds {} brushes, so its output must be a folder, not {}",
                    set.file_name,
                    set.brushes.len(),
                    path.display()
                )));
            }
            None => PathBuf::from(&title),
        };
        std::fs::create_dir_all(&dir)?;
        let mut taken = HashSet::new();
        let mut written = Vec::new();
        for brush in &set.brushes {
            let stem = unique(&mut taken, &safe_stem(&brush.name, "Brush"));
            let path = dir.join(format!("{stem}.{}", format.extension()));
            let bytes = one_brush(brush, format)?;
            std::fs::write(&path, bytes)?;
            written.push(path);
        }
        return Ok(written);
    }
    let path = output_path(out, &title, format.extension());
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)?;
    }
    let first = set.brushes.first().ok_or_else(|| Error::new("there are no brushes to write"))?;
    let bytes = match format {
        OutputFormat::Brush => brush::write_one(first, author_of(first))?,
        OutputFormat::BrushSet => brush::write_set(&title, &set.brushes)?,
        OutputFormat::Abr => abr::write(&set.brushes)?,
        OutputFormat::Kpp => krita::write_preset(first)?,
        OutputFormat::Bundle => krita::write_bundle(&title, &set.brushes)?,
        OutputFormat::Gbr => gbr::write(first)?,
        OutputFormat::Json => json::write(set)?,
    };
    std::fs::write(&path, bytes)?;
    Ok(vec![path])
}

fn one_brush(brush: &Brush, format: OutputFormat) -> Result<Vec<u8>> {
    match format {
        OutputFormat::Brush => brush::write_one(brush, author_of(brush)),
        OutputFormat::Kpp => krita::write_preset(brush),
        OutputFormat::Gbr => gbr::write(brush),
        _ => Err(Error::new("that format holds all brushes in one file")),
    }
}

fn author_of(brush: &Brush) -> &str {
    brush.author.as_ref().map(|a| a.name.as_str()).unwrap_or("")
}

fn has_extension(path: &Path) -> bool {
    path.extension().is_some_and(|e| !e.is_empty())
}

/// Where a one-file output goes: `out` itself if it names a file, a file called `title.ext` in it
/// if it names a folder, or `title.ext` in the working folder.
fn output_path(out: Option<&Path>, title: &str, ext: &str) -> PathBuf {
    match out {
        Some(path) if path.is_dir() => path.join(format!("{title}.{ext}")),
        Some(path) if has_extension(path) => path.to_path_buf(),
        Some(path) => path.join(format!("{title}.{ext}")),
        None => PathBuf::from(format!("{title}.{ext}")),
    }
}

/// `name` made safe as a file name: path separators and characters Windows doesn't take replaced,
/// trimmed, and no longer than 80 characters.
pub fn safe_stem(name: &str, fallback: &str) -> String {
    let mut out = String::new();
    for c in name.trim().chars() {
        if out.chars().count() >= 80 {
            break;
        }
        let c = match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' | '\0' => '_',
            c if (c as u32) < 0x20 => '_',
            c => c,
        };
        out.push(c);
    }
    let out = out.trim_matches(|c: char| c == '.' || c.is_whitespace()).to_owned();
    if out.is_empty() { fallback.to_owned() } else { out }
}

/// `stem`, or `stem-2`, `stem-3`… whichever `taken` doesn't have yet (by lower case).
pub(crate) fn unique(taken: &mut HashSet<String>, stem: &str) -> String {
    if taken.insert(stem.to_lowercase()) {
        return stem.to_owned();
    }
    for n in 2..1000 {
        let candidate = format!("{stem}-{n}");
        if taken.insert(candidate.to_lowercase()) {
            return candidate;
        }
    }
    stem.to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_made_safe() {
        assert_eq!(safe_stem("Soft / Round", "Brush"), "Soft _ Round");
        assert_eq!(safe_stem("  ", "Brush"), "Brush");
        assert_eq!(safe_stem("a:b*c?", "Brush"), "a_b_c_");
        assert_eq!(safe_stem("...", "Brush"), "Brush");
    }

    #[test]
    fn formats_are_named() {
        assert_eq!(OutputFormat::from_name("Photoshop"), Some(OutputFormat::Abr));
        assert_eq!(OutputFormat::from_name("krita"), Some(OutputFormat::Kpp));
        assert_eq!(OutputFormat::from_extension(".kpp"), Some(OutputFormat::Kpp));
        assert_eq!(OutputFormat::from_name("nope"), None);
    }
}
