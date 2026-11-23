//! Krita's `.kpp` presets and `.bundle` resource bundles.
//!
//! A `.kpp` is a PNG whose text chunk `preset` holds the preset's XML, and its tip is either one
//! Krita draws (a round `auto_brush`) or a PNG kept in the preset's own `<resources>`. This writer
//! keeps the tip as an embedded PNG (white, alpha its grey image), which Krita 4 and later read.

use std::collections::HashSet;

use procraft_engine::base64;
use procraft_engine::brush::{Brush, PressureCurve};
use procraft_engine::curve::Curve;
use procraft_engine::zip::ZipWriter;

use crate::Result;
use crate::images;
use crate::md5;
use crate::preview;
use crate::write::{safe_stem, unique};

/// A dual brush's second, as Krita's masking brush.
struct Masking {
    definition: String,
    opacity: f32,
    flow: f32,
    coeff: f32,
    op: &'static str,
}

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
    // a dual brush's second: Krita masks one preset with another, its own settings under
    // `MaskingBrush/Preset/`
    let mut masking = None;
    let mut mask_png = None;
    if let Some(second) = brush.secondary.as_deref() {
        let (definition, resource) = if second.round_tip() {
            (mask_definition(second), None)
        } else {
            let (_, pixels) = images::tip_gray(second);
            let png = images::alpha_png(&pixels)?;
            let file = format!("{}-mask.png", safe_stem(&second.name, "Brush"));
            let sum = md5::hex(&png);
            let definition = masked_definition(&file, &sum, second);
            (definition, Some((file, sum, png)))
        };
        mask_png = resource;
        masking = Some(Masking {
            definition,
            opacity: second.max_opacity.clamp(0.0, 1.0),
            flow: second.flow.clamp(0.0, 1.0),
            coeff: (second.max_size / brush.max_size.max(0.5)).clamp(0.01, 10.0),
            op: masking_op(brush.setting("dual_blend") as u32),
        });
    }
    let mut resources = String::new();
    resources.push_str(&format!(
        "<resource type=\"brushes\" filename=\"{}\" name=\"{}\" md5sum=\"{}\">{}</resource>\n",
        xml(&tip_file),
        xml(&brush.name),
        tip_md5,
        base64::encode(&tip_png)
    ));
    if let Some((file, sum, png)) = &mask_png {
        resources.push_str(&format!(
            "<resource type=\"brushes\" filename=\"{}\" name=\"{}\" md5sum=\"{}\">{}</resource>\n",
            xml(file),
            xml(file),
            sum,
            base64::encode(png)
        ));
    }
    if let Some((grain_name, grain_md5, grain_png)) = &grain {
        resources.push_str(&format!(
            "<resource type=\"patterns\" filename=\"{}\" name=\"{}\" md5sum=\"{}\">{}</resource>\n",
            xml(grain_name),
            xml(grain_name),
            grain_md5,
            base64::encode(grain_png)
        ));
    }
    let text = preset_xml(
        brush,
        &tip_file,
        &tip_md5,
        grain.as_ref().map(|(n, s, _)| (n.as_str(), s.as_str())),
        masking.as_ref(),
        &resources,
    );
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
    // the bundle's own thumbnail, as Krita's resource manager shows it
    if let Some(first) = brushes.first()
        && let Ok(preview) = preview::stroke_sized(first, 200, 200)
    {
        zip.add("preview.png", &preview);
        manifest.push_str(&format!(
            " <manifest:file-entry manifest:full-path=\"preview.png\" manifest:media-type=\"image/png\" manifest:md5sum=\"{}\"/>\n",
            md5::hex(&preview)
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
/// Latin-1 text chunk keeps it whole.
fn preset_xml(
    brush: &Brush,
    tip_file: &str,
    tip_md5: &str,
    grain: Option<(&str, &str)>,
    masking: Option<&Masking>,
    resources: &str,
) -> String {
    let name = xml(&brush.name);
    let spacing = brush.spacing.clamp(0.01, 10.0);
    let pressure_size = brush.pressure_size.clamp(0.0, 1.0);
    let pressure_opacity = brush.pressure_opacity.clamp(0.0, 1.0);
    // Krita turns its tips counter-clockwise on the canvas, ours clockwise
    let angle = (-brush.shape.rotation).rem_euclid(std::f32::consts::TAU);
    let definition = format!(
        "<Brush type=\"png_brush\" filename=\"{}\" name=\"{}\" md5sum=\"{}\" spacing=\"{}\" \
         angle=\"{}\" scale=\"1\" useAutoSpacing=\"0\" ColorAsMask=\"0\" brushApplication=\"1\"/>",
        xml(tip_file),
        name,
        tip_md5,
        number(spacing),
        number(angle)
    );
    let mut params = String::new();
    let mut push = |key: &str, value: &str| {
        params.push_str(&format!("<param type=\"string\" name=\"{}\">{}</param>\n", xml(key), xml(value)));
    };
    push("brush_definition", &definition);
    // the size in pixels: Krita keeps thousandths of one, the older diameter beside it
    push("SizeValue", &number(brush.max_size / 1000.0));
    push("diameter", &number(brush.max_size));
    let least = (brush.min_size / brush.max_size).clamp(0.0, 1.0);
    if pressure_size > 0.0 {
        push("PressureSize", "true");
        push("SizeUseCurve", "true");
        push("SizeSensor", &sensor(&brush.size_curve, least));
    } else {
        push("PressureSize", "false");
        push("SizeUseCurve", "false");
    }
    push("PressureOpacity", "true");
    if pressure_opacity > 0.0 {
        push("OpacityUseCurve", "true");
        push("OpacitySensor", &sensor(&Curve::linear(), 1.0 - pressure_opacity));
    } else {
        push("OpacityUseCurve", "false");
    }
    push("OpacityValue", &number(brush.max_opacity.clamp(0.0, 1.0)));
    push("FlowValue", &number(brush.flow.clamp(0.0, 1.0)));
    // its flow by the pressure: along its own curve, or straight where the brush model says so
    // without keeping one
    let flow_sensor = brush
        .pressure_curves
        .get(PressureCurve::Flow)
        .map(|curve| sensor(curve, 0.0))
        .or_else(|| (brush.setting("pressure_flow") > 0.0).then(|| sensor(&Curve::linear(), 0.0)));
    match flow_sensor {
        Some(sensor) => {
            push("PressureFlow", "true");
            push("FlowUseCurve", "true");
            push("FlowSensor", &sensor);
        }
        None => {
            push("PressureFlow", "false");
            push("FlowUseCurve", "false");
        }
    }
    push("SpacingValue", "1");
    // build-up (1) rather than wash, as ProCraft paints
    push("PaintOpAction", "1");
    // a tip softer than hard: Krita's softness is 1 less ours
    let softness = (1.0 - brush.hardness).clamp(0.0, 1.0);
    if softness > 0.0 {
        push("SoftnessValue", &number(softness));
        push("PressureSoftness", "false");
    }
    // flattened across, by the pen too where its roundness curve says so
    if brush.shape.roundness < 1.0 {
        push("RatioValue", &number(brush.shape.roundness.clamp(0.01, 1.0)));
        match brush.pressure_curves.get(PressureCurve::Roundness) {
            Some(curve) if brush.setting("pressure_roundness") > 0.0 => {
                push("PressureRatio", "true");
                push("RatioUseCurve", "true");
                push("RatioSensor", &sensor(curve, 1.0));
            }
            _ => push("PressureRatio", "false"),
        }
    }
    // its colour turned at random each stamp: Krita's h, s and v jitter
    for (letter, key) in [("h", "stamp_hue"), ("s", "stamp_saturation"), ("v", "stamp_lightness")] {
        let amount = brush.setting(key);
        if amount > 0.0 {
            push(&format!("{letter}Value"), &number(amount));
            push(&format!("Pressure{letter}"), "true");
            push(&format!("{letter}UseCurve"), "false");
            push(&format!("{letter}Sensor"), "<params id=\"fuzzy\"><curve>0,0;1,1</curve></params>");
        }
    }
    // stamps spread along and across the stroke: our jitter's controls as the distances they throw
    let reach = 0.5 * (1.0 + brush.shape.roundness * brush.shape.roundness).sqrt();
    let spread = |control: f32| reach * procraft_engine::stroke::JITTER_FILE_SCALE * control.powf(2.2);
    let along = spread(brush.setting("jitter_linear"));
    let across = spread(brush.setting("jitter_lateral"));
    if along > 0.0 || across > 0.0 {
        push("ScatterValue", &number(along.max(across)));
        push("PressureScatter", "false");
        push("ScatterUseCurve", "false");
        push("Scattering/AxisX", if along > 0.0 { "true" } else { "false" });
        push("Scattering/AxisY", if across > 0.0 { "true" } else { "false" });
    }
    for (key, flag) in [("HorizontalMirrorEnabled", "flip_x"), ("VerticalMirrorEnabled", "flip_y")] {
        if brush.setting(flag) > 0.0 {
            push(key, "true");
        }
    }
    if let Some((grain_file, grain_md5)) = grain {
        push("Texture/Pattern/Enabled", "true");
        push("Texture/Pattern/PatternFileName", grain_file);
        push("Texture/Pattern/PatternName", grain_file);
        push("Texture/Pattern/PatternMD5Sum", grain_md5);
        push("Texture/Pattern/Scale", &number(brush.grain.scale.clamp(0.01, 16.0)));
        push("Texture/Pattern/Strength", &number(brush.grain.depth.clamp(0.0, 1.0)));
        push("Texture/Pattern/TexturingMode", "0");
    }
    if let Some(mask) = masking {
        push("MaskingBrush/Enabled", "true");
        push("MaskingBrush/MaskingCompositeOp", mask.op);
        push("MaskingBrush/UseMasterSize", "true");
        push("MaskingBrush/MasterSizeCoeff", &number(mask.coeff));
        push("MaskingBrush/Preset/paintop", "paintbrush");
        push("MaskingBrush/Preset/brush_definition", &mask.definition);
        push("MaskingBrush/Preset/OpacityValue", &number(mask.opacity));
        push("MaskingBrush/Preset/OpacityUseCurve", "false");
        push("MaskingBrush/Preset/FlowValue", &number(mask.flow));
        push("MaskingBrush/Preset/FlowUseCurve", "false");
    }
    drop(push);
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<Preset paintopid=\"paintbrush\" name=\"{name}\">\n\
         {params}<resources>\n{resources}</resources>\n</Preset>\n"
    )
}

/// A masking brush as a round one: its diameter in pixels, its spacing, its roundness.
fn mask_definition(second: &Brush) -> String {
    format!(
        "<Brush type=\"auto_brush\" scale=\"1\" useAutoSpacing=\"0\" BrushVersion=\"2\" \
         spacing=\"{}\" angle=\"0\" brushApplication=\"0\"><MaskGenerator diameter=\"{}\" \
         hfade=\"0\" vfade=\"0\" id=\"default\" spikes=\"2\" type=\"circle\" ratio=\"{}\" \
         antialiasEdges=\"1\"/></Brush>",
        number(second.spacing.clamp(0.01, 10.0)),
        number(second.max_size.max(1.0)),
        number(second.shape.roundness.clamp(0.01, 1.0))
    )
}

/// A masking brush as one of its own PNG tips.
fn masked_definition(file: &str, sum: &str, second: &Brush) -> String {
    format!(
        "<Brush type=\"png_brush\" filename=\"{}\" md5sum=\"{}\" scale=\"1\" useAutoSpacing=\"0\" \
         spacing=\"{}\" angle=\"0\" ColorAsMask=\"0\" brushApplication=\"0\"/>",
        xml(file),
        sum,
        number(second.spacing.clamp(0.01, 10.0))
    )
}

/// A blend mode's number as Krita names it, for the masking brush's op.
fn masking_op(id: u32) -> &'static str {
    use procraft_engine::document::BlendMode as B;
    match B::from_id(id).unwrap_or(B::Multiply) {
        B::Normal => "normal",
        B::Multiply => "multiply",
        B::Darken | B::DarkerColor => "darken",
        B::ColorBurn => "burn",
        B::LinearBurn => "linear_burn",
        B::Lighten | B::LighterColor => "lighten",
        B::Screen => "screen",
        B::Add => "linear_dodge",
        B::ColorDodge => "dodge",
        B::Overlay => "overlay",
        B::SoftLight => "soft_light",
        B::HardLight => "hard_light",
        B::VividLight => "vivid_light",
        B::LinearLight => "linear_light",
        B::PinLight => "pin_light",
        B::HardMix => "hard_mix",
        B::Difference => "difference",
        B::Exclusion => "exclusion",
        B::Subtract => "subtract",
        B::Divide => "divide",
        B::Hue => "hue",
        B::Saturation => "saturation",
        B::Color => "color",
        B::Luminosity => "luminosity",
        B::Shade => "multiply",
    }
}

/// A curve of the pen's as Krita keeps one, from `least` to 1 when the brush's own curve says
/// nothing at its ends.
fn sensor(curve: &Curve, least: f32) -> String {
    let mut text = String::new();
    for point in curve.points() {
        if !text.is_empty() {
            text.push(';');
        }
        text.push_str(&format!("{},{}", number(point.x), number(point.y)));
    }
    if curve.points().len() < 2 {
        text = format!("0,{};1,1", number(least));
    }
    format!("<params id=\"sensorslist\"><ChildSensor id=\"pressure\"><curve>{text}</curve></ChildSensor></params>")
}

/// A number for XML: no exponent, at most six places after the point (a size is kept in
/// thousandths, so three would lose its last two digits).
fn number(value: f32) -> String {
    let value = if value.is_finite() { value } else { 0.0 };
    let text = format!("{value:.6}");
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::read_bytes;

    fn near(a: f32, b: f32, tolerance: f32) -> bool {
        (a - b).abs() <= tolerance
    }

    #[test]
    fn a_bundle_keeps_the_size_and_the_settings_krita_had() {
        let mut brush = Brush::round_pen();
        brush.name = "Wide Marker".to_owned();
        brush.max_size = 144.22;
        brush.min_size = 1.44;
        brush.hardness = 0.357;
        brush.spacing = 0.1;
        brush.shape.roundness = 0.828;
        brush.shape.rotation = 2.095;
        brush.set_setting("jitter_linear", 0.46);
        brush.set_setting("flip_y", 1.0);
        let bytes = write_bundle("Test", std::slice::from_ref(&brush)).expect("a bundle");
        assert!(bytes.len() > 1000);
        let set = read_bytes(&bytes, "Test.bundle").expect("the bundle back");
        let back = set.brushes.first().expect("one brush");
        assert!(near(back.max_size, 144.22, 0.02), "{}", back.max_size);
        assert!(near(back.min_size, 1.44, 0.02), "{}", back.min_size);
        assert!(near(back.hardness, 0.357, 0.002), "{}", back.hardness);
        assert!(near(back.shape.roundness, 0.828, 0.002), "{}", back.shape.roundness);
        assert!(near(back.shape.rotation, 2.095, 0.002), "{}", back.shape.rotation);
        assert!(near(back.spacing, 0.1, 0.001), "{}", back.spacing);
        assert!(back.setting("jitter_linear") > 0.3, "{}", back.setting("jitter_linear"));
        assert_eq!(back.setting("flip_y"), 1.0);
    }

    #[test]
    fn a_dual_brush_is_kept_as_a_masking_brush() {
        let mut brush = Brush::round_pen();
        brush.name = "Masked".to_owned();
        brush.max_size = 100.0;
        let mut second = Brush::round_pen();
        second.name = "Mask".to_owned();
        second.max_size = 50.0;
        second.max_opacity = 0.5;
        brush.secondary = Some(Box::new(second));
        brush.set_setting("dual_blend", 1.0);
        let bytes = write_bundle("Dual", std::slice::from_ref(&brush)).expect("a bundle");
        let set = read_bytes(&bytes, "Dual.bundle").expect("the bundle back");
        let back = set.brushes.first().expect("one brush");
        let dual = back.secondary.as_deref().expect("its second");
        assert!(near(dual.max_size, 50.0, 0.5), "{}", dual.max_size);
        // Krita masks in wash mode: the second's opacity is the most its flow builds up to
        let most = dual.pressure_curves.get(PressureCurve::Flow).expect("its flow's curve");
        assert!(near(most.eval(1.0), 0.5, 0.001), "{}", most.eval(1.0));
        assert_eq!(dual.setting("pressure_flow"), 1.0);
    }
}
