//! Writing PaintTool SAI2 custom tools (`.saitdat`).
//!
//! The key order is the one SAI2 2021 writes itself, taken from the templates the Brush Bridge
//! project (MIT) recorded from its `init` folders. A tool names its brush form and texture by
//! file name; those bitmaps live in SAI2's own settings folder, and we ship none, so the file asks
//! for SAI2's default round form and no texture. SAI2 keeps its tools in
//! `Documents\SYSTEMAX Software Development\SAIv2\settings\custool\{nrm,ink,shp}`; a file put
//! there is usable from its tool palette, while a group entry (`_N.saitgrp`) decides where it sits.

use procraft_engine::brush::Brush;

use crate::Result;

const MODSTR: i64 = 1836216174;

fn push_line(out: &mut String, key: &str, kind: char, value: &str) {
    out.push_str(key);
    out.push('=');
    out.push(kind);
    out.push(':');
    out.push_str(value);
    out.push('\n');
}

/// One parameter: its key, its SAI2 type, and its value.
type Param = (&'static str, char, String);

fn param(key: &'static str, value: impl ToString) -> Param {
    (key, 'I', value.to_string())
}

fn text(key: &'static str, value: impl ToString) -> Param {
    (key, 'U', value.to_string())
}

/// The parameters a SAI2 brush tool carries, in SAI2's own order.
fn brush_params(brush: &Brush, wet: bool) -> Vec<Param> {
    let size = brush.max_size.max(0.5);
    let least = if size > 0.0 { brush.min_size / size } else { 0.0 };
    let share = |value: f32| (value.clamp(0.0, 1.0) * 100.0).round() as i64;
    let angle = brush.shape.rotation.to_degrees().rem_euclid(360.0).round() as i64;
    let spacing = (brush.spacing * 100.0).round().clamp(1.0, 200.0) as i64;
    let mut params: Vec<Param> = vec![
        ("modstr", 'I', MODSTR.to_string()),
        param("wavno", (brush.hardness.clamp(0.0, 1.0) * 4.0).round() as i64),
        param("curszscl", 1),
        param("cursize", (size * 10.0).round() as i64),
        param("minsize", share(least)),
        param("curdens", share(brush.max_opacity)),
        param("mindens", share(brush.min_opacity)),
        param("fomgid", 0),
        param("fomcat", 0),
        text("fomnam", ""),
        param("blotval", 50),
        param("blotscl", 100),
        param("blotinvf", 0),
        param("blotivtf", 1),
        param("blotsharpf", 0),
        param("brslmin", 0),
        param("brslsiz", 50),
        param("brslrnd", 0),
        param("brsldir", 1),
        param("brslturnf", 1),
        param("scatden", 100),
        param("scatanc", i64::from(brush.shape.follow > 0.0)),
        param("scatang", angle),
        param("scatanj", 0),
        param("scatscl", 100),
        param("scatszj", share(brush.setting("jitter_size"))),
        param("scatspc", spacing),
        param("scatsct", share(brush.setting("scatter"))),
        param("scathqsf", 0),
    ];
    params.extend([
        param("texgid", 0),
        param("texcat", 0),
        text("texnam", ""),
        param("texval", 95),
        param("texscl", 100),
        param("texscr", 0),
        param("texinvf", 0),
        param("texivtf", 1),
    ]);
    if wet {
        params.extend([
            param("blend", share(brush.setting("wet_pull"))),
            param("dilution", share(brush.setting("wet_dilution"))),
            param("persist", share(brush.setting("wet_charge"))),
            param("keepaf", 0),
            param("blurthres", 50),
        ]);
    }
    params.extend([
        param("hardness", share(brush.hardness)),
        param("densamp", 0),
        param("v1dnprsf", 0),
        param("flattenf", 1),
        param("szprsf", i64::from(brush.pressure_size > 0.0)),
        param("dnprsf", i64::from(brush.pressure_opacity > 0.0)),
    ]);
    if wet {
        params.extend([param("blprsf", 0), param("dlprsf", i64::from(brush.setting("pressure_flow") > 0.0))]);
    }
    params.extend([
        param("szsens", 100),
        param("dnsens", 100),
        param("sbpos_patlist", 0),
        param("previewbg", 0),
        param("showf_fom", 0),
        param("showf_tex", 0),
    ]);
    params
}

/// Writes one brush as a SAI2 custom tool.
pub fn write(brush: &Brush) -> Result<Vec<u8>> {
    let name = if brush.name.trim().is_empty() { "Brush".to_owned() } else { brush.name.trim().to_owned() };
    // a brush with water in it is SAI2's water tool; a dry one is its brush
    let wet = brush.mixes_wet();
    let tool = if wet { "water" } else { "brush" };
    let header = [
        ("tidstr", 'S', tool.to_owned()),
        ("xidstr", 'S', String::new()),
        param("vkc", 0),
        param("tabflt", 0),
        param("tfmode", 0),
        param("bzintf", 1),
        text("name", name.as_str()),
        text("addt", "brushconv"),
    ];
    let mut out = String::new();
    for (key, kind, value) in header.iter() {
        push_line(&mut out, key, *kind, value);
    }
    out.push_str(".\n");
    for (key, kind, value) in brush_params(brush, wet).iter() {
        push_line(&mut out, key, *kind, value);
    }
    out.push_str(".\n.\n.\n--EOF--\n");
    Ok(out.into_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_a_tool_its_own_reader_takes_back() {
        let mut brush = Brush { name: "Water soft".into(), ..Brush::round_pen() };
        brush.spacing = 0.25;
        brush.max_size = 200.0;
        brush.min_size = 60.0;
        brush.max_opacity = 0.8;
        brush.min_opacity = 0.2;
        brush.hardness = 0.25;
        brush.set_setting("wet_dilution", 0.4);
        brush.set_setting("wet_pull", 0.5);
        brush.set_setting("wet_charge", 0.25);
        let bytes = write(&brush).expect("write");
        let text = String::from_utf8(bytes.clone()).expect("text");
        assert!(text.starts_with("tidstr=S:water\n"), "{}", text.lines().next().unwrap_or(""));
        assert!(text.contains("cursize=I:2000\n"));
        assert!(text.contains("minsize=I:30\n"));
        assert!(text.contains("dilution=I:40\n"));
        let tool = procraft_engine::sai::read(&bytes).expect("read back");
        assert_eq!(tool.name, "Water soft");
        assert_eq!(tool.tool, "water");
        assert_eq!(tool.size, Some(200.0));
        assert!((tool.min_size - 0.3).abs() < 1e-6);
        assert!((tool.density - 0.8).abs() < 1e-6);
        assert!((tool.min_density - 0.2).abs() < 1e-6);
        assert!((tool.hardness - 0.25).abs() < 1e-6);
        assert!((tool.dilution - 0.4).abs() < 1e-6);
        assert!((tool.blend - 0.5).abs() < 1e-6);
    }

    #[test]
    fn a_dry_brush_is_written_as_a_brush_tool() {
        let brush = Brush { name: "Dry".into(), ..Brush::round_pen() };
        let bytes = write(&brush).expect("write");
        let text = String::from_utf8(bytes).expect("text");
        assert!(text.starts_with("tidstr=S:brush\n"));
        assert!(!text.contains("dilution="));
    }
}
