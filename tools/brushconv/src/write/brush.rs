//! ProCraft's own `.brush` and `.brushset`.

use procraft_engine::brush::{Brush, MaterialSource};
use procraft_engine::brushfile::{self, Pictures as FilePictures};
use procraft_engine::textures;

use crate::Result;
use crate::images;

/// One brush as a `.brush` file.
pub fn write_one(brush: &Brush, author: &str) -> Result<Vec<u8>> {
    let pictures = OwnedPictures::make(brush)?;
    Ok(brushfile::write(brush, author, &pictures.borrow()))
}

/// `brushes` as a `.brushset` file called `name`.
pub fn write_set(name: &str, brushes: &[Brush]) -> Result<Vec<u8>> {
    let mut pictures = Vec::with_capacity(brushes.len());
    for brush in brushes {
        pictures.push(OwnedPictures::make(brush)?);
    }
    let items: Vec<(&Brush, &str, FilePictures<'_>)> =
        brushes.iter().zip(&pictures).map(|(brush, pictures)| (brush, author_name(brush), pictures.borrow())).collect();
    Ok(brushfile::write_set(name, &items, |i| format!("{:02}", i + 1)))
}

fn author_name(brush: &Brush) -> &str {
    brush.author.as_ref().map(|a| a.name.as_str()).unwrap_or("")
}

/// The pictures a brush file needs, owned.
struct OwnedPictures {
    shape: Vec<u8>,
    grain: Option<Vec<u8>>,
    metallic: Option<Vec<u8>>,
    roughness: Option<Vec<u8>>,
    thumbnail: Vec<u8>,
    author: Option<Vec<u8>>,
    signature: Option<Vec<u8>>,
    secondary: Option<Box<OwnedPictures>>,
}

impl OwnedPictures {
    fn make(brush: &Brush) -> Result<OwnedPictures> {
        let (side, pixels) = images::tip_gray(brush);
        let grain = match images::grain_gray(brush) {
            Some((side, pixels)) => Some(images::gray_png(side, &pixels)?),
            None => None,
        };
        let author = brush.author.as_ref().and_then(|a| png_of(a.picture.as_deref()));
        let signature = brush.author.as_ref().and_then(|a| png_of(a.signature.as_deref()));
        Ok(OwnedPictures {
            shape: images::gray_png(side, &pixels)?,
            grain,
            metallic: material_png(&brush.metallic)?,
            roughness: material_png(&brush.roughness)?,
            thumbnail: images::thumbnail_png(brush),
            author,
            signature,
            secondary: match &brush.secondary {
                Some(second) => Some(Box::new(OwnedPictures::make(second)?)),
                None => None,
            },
        })
    }

    fn borrow(&self) -> FilePictures<'_> {
        FilePictures {
            shape_png: &self.shape,
            grain_png: self.grain.as_deref(),
            metallic_png: self.metallic.as_deref(),
            roughness_png: self.roughness.as_deref(),
            thumbnail_png: &self.thumbnail,
            author_png: self.author.as_deref(),
            signature_png: self.signature.as_deref(),
            secondary: self.secondary.as_ref().map(|s| Box::new(s.borrow())),
        }
    }
}

fn png_of(bytes: Option<&[u8]>) -> Option<Vec<u8>> {
    bytes.filter(|b| b.starts_with(&[0x89, b'P', b'N', b'G'])).map(<[u8]>::to_vec)
}

fn material_png(source: &MaterialSource) -> Result<Option<Vec<u8>>> {
    let pixels = if let Some(source) = &source.source {
        Some((textures::TEXTURE_SIZE as u32, source.pixels.to_vec()))
    } else {
        source.image.and_then(textures::grain_image).map(|g| (textures::TEXTURE_SIZE as u32, g.pixels))
    };
    match pixels {
        Some((side, pixels)) => Ok(Some(images::gray_png(side, &pixels)?)),
        None => Ok(None),
    }
}
