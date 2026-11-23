//! Brush format conversion on top of ProCraft's brush readers and writers.
//!
//! Reads the brush files ProCraft reads (its own `.brush`/`.brushset`, Photoshop `.abr`, Krita
//! `.kpp`/`.bundle`, GIMP `.gbr`/`.gih`/`.vbr`, Clip Studio `.sut`/`.sutg`) into ProCraft's
//! [`Brush`] model, and writes that model out as any of them.

mod error;
pub mod fields;
mod images;
mod md5;
mod preview;
mod read;
mod write;

pub use error::{Error, Result};
pub use read::{SourceFormat, detect_format, read_bytes, read_path};
pub use write::{OutputFormat, write_set};

use procraft_engine::brush::Brush;

/// A named group of brushes, as one input file held them.
#[derive(Clone, Debug)]
pub struct BrushSet {
    pub name: String,
    pub brushes: Vec<Brush>,
    /// What kind of file it came from, for reports.
    pub source: SourceFormat,
    /// The input's file name (without folders), for default output names.
    pub file_name: String,
}

impl BrushSet {
    pub fn is_empty(&self) -> bool {
        self.brushes.is_empty()
    }
}
