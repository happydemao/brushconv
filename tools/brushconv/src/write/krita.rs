//! Krita's `.kpp` presets and `.bundle` resource bundles.
//!
//! A `.kpp` is a PNG whose text chunk `preset` holds the preset's XML, and its tip is either one
//! Krita draws (a round `auto_brush`) or a PNG kept in the preset's own `<resources>`. This writer
//! keeps the tip as an embedded PNG (white, alpha its grey image), which Krita 4 and later read.

use std::collections::HashSet;

use procraft_engine::base64;
use procraft_engine::brush::Brush;
use procraft_engine::zip::ZipWriter;

use crate::Result;
use crate::images;
use crate::md5;
use crate::preview;
use crate::write::{safe_stem, unique};

/// One brush as a `.kpp` preset.
pub fn write_preset(brush: &Brush) -> Result<Vec<u8>> {
    let (width, height, rgba) = match preview::stroke_rgba(brush) {
        Ok((w, h, pixels)) => (w, h, pixels),
        Err(_) => (1, 1, vec![0, 0, 0, 0]),
    };
    let (_tip_side, tip) = images::tip_gray(brush);
    let tip_png = images::alpha_png(&tip)?;
    let tip_md5 = md5::hex(&tip_png);
    let tip_file = format!("{}.png", safe_stem(&brush.name, "Brush"));
    let grain = match images::grain_gray(brush) {
        Some((side, pixels)) => {
            let png = images::gray_png(side, &pixels)?;
            let name = format!("{}-grain.png", safe_stem(&brush.name, "Brush"));
            let sum = md5::hex(&png);
            Some((name, sum, png))
        }
        None => None,
    };
    let mut resources = String::new();
    resources.push_str(&format!(
        "<resource type=\"brushes\" filename=\"{}\" name=\"{}\" md5sum=\"{}\">{}</resource>\n",
        xml(&tip_file),
        xml(&brush.name),
        tip_md5,
        base64::encode(&tip_png)
    ));
    if let Some((grain_name, grain_md5, grain_png)) = &grain {
        resources.push_str(&format!(
            "<resource type=\"patterns\" filename=\"{}\" name=\"{}\" md5sum=\"{}\">{}</resource>\n",
            xml(grain_name),
            xml(grain_name),
            grain_md5,
            base64::encode(grain_png)
        ));
    }
    let text =
        preset_xml(brush, &tip_file, &tip_md5, grain.as_ref().map(|(n, s, _)| (n.as_str(), s.as_str())), &resources);
    preset_png(width, height, &rgba, &text)
}

/// `brushes` as a `.bundle` called `name`.
pub fn write_bundle(name: &str, brushes: &[Brush]) -> Result<Vec<u8>> {
    let mut zip = ZipWriter::new();
    zip.add("mimetype", b"application/x-krita-resourcebundle");
    let title = xml(name);
    zip.add(
        "meta.xml",
        format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<meta><title>{title}</title></meta>\n").as_bytes(),
    );
    let mut taken = HashSet::new();
    let mut manifest = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <manifest:manifest xmlns:manifest=\"urn:oasis:names:tc:opendocument:xmlns:manifest:1.0\" \
         manifest:version=\"1.2\">\n",
    );
    for brush in brushes {
        let stem = unique(&mut taken, &safe_stem(&brush.name, "Brush"));
        let file = format!("paintoppresets/{stem}.kpp");
        let bytes = write_preset(brush)?;
        zip.add(&file, &bytes);
        manifest.push_str(&format!(
            " <manifest:file-entry manifest:full-path=\"{}\" manifest:md5sum=\"{}\"/>\n",
            xml(&file),
            md5::hex(&bytes)
        ));
    }
    manifest.push_str("</manifest:manifest>\n");
    zip.add("META-INF/manifest.xml", manifest.as_bytes());
    Ok(zip.finish())
}

/// The preset's PNG: the stroke preview with the preset's XML in its text chunks.
fn preset_png(width: u32, height: u32, rgba: &[u8], xml: &str) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut out, width.max(1), height.max(1));
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header()?;
        writer.write_text_chunk(&png::text_metadata::TEXtChunk::new("version", "5.0"))?;
        let mut preset = png::text_metadata::ZTXtChunk::new("preset", xml);
        preset.compress_text()?;
        writer.write_text_chunk(&preset)?;
        writer.write_image_data(rgba)?;
        writer.finish()?;
    }
    Ok(out)
}

/// The preset's XML, ASCII only (non-ASCII characters are character references) so that the
/// Latin-1 zTXt chunk keeps it whole.
fn preset_xml(brush: &Brush, tip_file: &str, tip_md5: &str, grain: Option<(&str, &str)>, resources: &str) -> String {
    let name = xml(&brush.name);
    let spacing = brush.spacing.clamp(0.01, 10.0);
    let pressure_size = brush.pressure_size.clamp(0.0, 1.0);
    let pressure_opacity = brush.pressure_opacity.clamp(0.0, 1.0);
    let definition = format!(
        "<Brush type=\"png_brush\" filename=\"{}\" name=\"{}\" md5sum=\"{}\" spacing=\"{}\" \
         angle=\"0\" scale=\"1\" useAutoSpacing=\"0\" ColorAsMask=\"0\" brushApplication=\"1\"/>",
        xml(tip_file),
        name,
        tip_md5,
        number(spacing)
    );
    let mut params = String::new();
    let mut push = |key: &str, value: &str| {
        params.push_str(&format!("<param type=\"string\" name=\"{}\">{}</param>\n", xml(key), xml(value)));
    };
    push("brush_definition", &definition);
    push("PressureSize", if pressure_size > 0.0 { "true" } else { "false" });
    if pressure_size > 0.0 {
        push("SizeUseCurve", "true");
        push("SizeSensor", &sensor_curve(1.0 - pressure_size, 1.0));
    }
    push("PressureOpacity", "true");
    if pressure_opacity > 0.0 {
        push("OpacityUseCurve", "true");
        push("OpacitySensor", &sensor_curve(1.0 - pressure_opacity, 1.0));
    } else {
        push("OpacityUseCurve", "false");
    }
    push("OpacityValue", &number(brush.max_opacity.clamp(0.0, 1.0)));
    push("FlowValue", &number(brush.flow.clamp(0.0, 1.0)));
    push("FlowUseCurve", "false");
    push("SpacingValue", "1");
    // build-up (1) rather than wash, as ProCraft paints
    push("PaintOpAction", "1");
    if let Some((grain_file, grain_md5)) = grain {
        push("Texture/Pattern/Enabled", "true");
        push("Texture/Pattern/PatternFileName", grain_file);
        push("Texture/Pattern/PatternName", grain_file);
        push("Texture/Pattern/PatternMD5Sum", grain_md5);
        push("Texture/Pattern/Scale", &number(brush.grain.scale.clamp(0.01, 16.0)));
        push("Texture/Pattern/Strength", &number(brush.grain.depth.clamp(0.0, 1.0)));
        push("Texture/Pattern/TexturingMode", "0");
    }
    drop(push);
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<Preset paintopid=\"paintbrush\" name=\"{name}\">\n\
         {params}<resources>\n{resources}</resources>\n</Preset>\n"
    )
}

fn sensor_curve(low: f32, high: f32) -> String {
    format!(
        "<params id=\"sensorslist\"><ChildSensor id=\"pressure\"><curve>0,{};1,{}</curve></ChildSensor></params>",
        number(low),
        number(high)
    )
}

/// A number for XML: no exponent, at most three places after the point.
fn number(value: f32) -> String {
    let value = if value.is_finite() { value } else { 0.0 };
    let text = format!("{value:.3}");
    let text = text.trim_end_matches('0').trim_end_matches('.');
    if text.is_empty() || text == "-0" { "0".to_owned() } else { text.to_owned() }
}

/// `text` as XML character data or an attribute value, all of it ASCII.
fn xml(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            c if (c as u32) > 0x7e => out.push_str(&format!("&#x{:X};", c as u32)),
            c => out.push(c),
        }
    }
    out
}
