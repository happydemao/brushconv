//! PaintTool SAI Ver.2's custom tools: the `.saitdat` files in its `custool` folders.
//!
//! A `.saitdat` is text: `key=T:value` lines grouped into sections separated by a lone `.`, ending
//! with `--EOF--`. Section 0 names the tool, section 1 holds its parameters, and section 2 holds a
//! second parameter set when `tfmode` is 2. The format was worked out by the Brush Bridge project
//! (MIT, https://github.com/CanderFlower/brush-bridge) from SAI2's own `init` folders; SAI2
//! publishes no specification.

/// One SAI2 custom tool, the parts ProCraft can show or convert.
#[derive(Clone, Debug, PartialEq)]
pub struct SaiTool {
    /// The tool's name, its `name` key.
    pub name: String,
    /// Its type, `tidstr`: `brush`, `water`, `pencil`, `airbrush`, `marker`, `eraser`, `scatter`…
    pub tool: String,
    /// The size in canvas pixels (`cursize` is in tenths of a pixel).
    pub size: Option<f32>,
    /// The least size as a share of the most (`minsize`, 0..1).
    pub min_size: f32,
    /// The density (`curdens`, 0..1).
    pub density: f32,
    /// The least density (`mindens`, 0..1).
    pub min_density: f32,
    /// The edge's hardness (`wavno`, 0..4, as its four steps are).
    pub hardness: f32,
    /// The brush form: its kind (1 blotmap, 2 bristle, 3 brshape, 4 scatter) and BMP name.
    pub form: Option<(u8, String)>,
    /// The texture: its BMP name, strength, scale, and whether it is inverted.
    pub texture: Option<(String, f32, f32, bool)>,
    /// How much the brush mixes with what is under it (0..1).
    pub blend: f32,
    /// Its water content (0..1).
    pub dilution: f32,
    /// How far its colour stretches (0..1).
    pub persist: f32,
    /// Whether pressure drives the size, the density, the mixing, or the water.
    pub pressure_size: bool,
    pub pressure_density: bool,
    pub pressure_blend: bool,
    pub pressure_dilution: bool,
    /// The spacing (0..1 of the stamp's size) the scatter shape asks for, when it is not a brush.
    pub spacing: Option<f32>,
    /// The scatter shape's scattering (0..1) and count.
    pub scatter: f32,
    pub count: f32,
    /// The fixed angle in degrees, and whether the stamp follows the stroke.
    pub angle: f32,
    pub angle_follow: bool,
    /// The stamp's width over its height (0.01..1), and how much it jitters and shrinks.
    pub roundness: f32,
    pub size_jitter: f32,
    /// Hue, saturation, brightness and "colour" jitter, 0..1 each.
    pub colour_jitter: [f32; 4],
}

/// Reads a `.saitdat`, or `None` when the bytes are not one.
pub fn read(bytes: &[u8]) -> Option<SaiTool> {
    let text = String::from_utf8_lossy(bytes);
    if !text.contains("tidstr=") && !text.contains("--EOF--") {
        return None;
    }
    let mut sections = Vec::new();
    let mut section: Vec<(String, String)> = Vec::new();
    for line in text.replace("\r\n", "\n").split('\n') {
        if line == "--EOF--" {
            break;
        }
        if line == "." {
            sections.push(std::mem::take(&mut section));
            continue;
        }
        let Some((key, rest)) = line.split_once('=') else { continue };
        // `T:value`, where T is S, U or I; a line with no type keeps the rest as it is.
        let value = match rest.split_once(':') {
            Some(("S" | "U" | "I", value)) => value,
            _ => rest,
        };
        section.push((key.to_owned(), value.to_owned()));
        if sections.len() >= 4 {
            break;
        }
    }
    if !section.is_empty() {
        sections.push(section);
    }
    let header = sections.first()?;
    let params = sections.get(1).unwrap_or(header);
    let get = |at: &[(String, String)], key: &str| at.iter().find(|(k, _)| k == key).map(|(_, v)| v.clone());
    let tool = get(header, "tidstr")?;
    let name = get(header, "name").unwrap_or_else(|| tool.clone());
    let number = |key: &str| get(params, key).and_then(|v| v.trim().parse::<f32>().ok());
    let flag = |key: &str| get(params, key).is_some_and(|v| v.trim() != "0");
    let share = |key: &str, scale: f32| number(key).map_or(0.0, |v| (v / scale).clamp(0.0, 1.0));
    let size = number("cursize").filter(|px| *px > 0.0).map(|tenths| tenths / 10.0);
    let form = get(params, "fomcat").and_then(|cat| {
        let kind = cat.trim().parse::<u8>().ok()?;
        let name = get(params, "fomnam")?;
        (kind != 0 && !name.trim().is_empty()).then_some((kind, name))
    });
    let texture = get(params, "texcat").and_then(|cat| {
        let kind = cat.trim().parse::<u8>().ok()?;
        let name = get(params, "texnam")?;
        (kind != 0 && !name.trim().is_empty()).then_some((
            name,
            share("texval", 100.0),
            number("texscl").map_or(1.0, |scale| (scale / 100.0).max(0.01)),
            flag("texinvf"),
        ))
    });
    Some(SaiTool {
        name: name.trim().to_owned(),
        tool: tool.trim().to_owned(),
        size,
        min_size: share("minsize", 100.0),
        density: share("curdens", 100.0),
        min_density: share("mindens", 100.0),
        hardness: number("wavno").map_or(0.5, |waves| (waves / 4.0).clamp(0.0, 1.0)),
        form,
        texture,
        blend: share("blend", 100.0),
        dilution: share("dilution", 100.0),
        persist: share("persist", 100.0),
        pressure_size: flag("szprsf"),
        pressure_density: flag("dnprsf"),
        pressure_blend: flag("blprsf"),
        pressure_dilution: flag("dlprsf"),
        spacing: number("scatspc").filter(|spacing| *spacing > 0.0).map(|percent| (percent / 100.0).clamp(0.0, 2.0)),
        scatter: share("scatsct", 100.0),
        count: number("scatcnt").map_or(1.0, |count| count.clamp(1.0, 20.0)),
        angle: number("scatang").unwrap_or(0.0),
        angle_follow: get(params, "scatanc").is_some_and(|control| control.trim() == "1"),
        roundness: number("scatwth").map_or(1.0, |width| (1.0 - (width.abs() / 100.0)).clamp(0.01, 1.0)),
        size_jitter: share("scatszj", 100.0),
        colour_jitter: [
            share("scathuej", 100.0),
            share("scatsatj", 100.0),
            share("scatbrij", 100.0),
            share("scatcolj", 100.0),
        ],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const TOOL: &str = "tidstr=S:brush\nxidstr=S:\nvkc=I:0\ntabflt=I:0\ntfmode=I:0\nbzintf=I:1\nname=U:Water soft\naddt=U:\n.\nmodstr=I:1836216174\nwavno=I:1\ncurszscl=I:1\ncursize=I:2000\nminsize=I:30\ncurdens=I:80\nmindens=I:20\nfomcat=I:3\nfomnam=U:soft-shape\ntexcat=I:2\ntexnam=U:paper-01\ntexval=I:30\ntexscl=I:100\ntexinvf=I:0\nblend=I:50\ndilution=I:40\npersist=I:25\nszprsf=I:1\ndnprsf=I:1\nblprsf=I:0\ndlprsf=I:1\n.\n.\n--EOF--\n";

    #[test]
    fn reads_a_custom_tool() {
        let tool = read(TOOL.as_bytes()).expect("a tool");
        assert_eq!(tool.name, "Water soft");
        assert_eq!(tool.tool, "brush");
        assert_eq!(tool.size, Some(200.0));
        assert!((tool.min_size - 0.3).abs() < 1e-6);
        assert!((tool.density - 0.8).abs() < 1e-6);
        assert!((tool.hardness - 0.25).abs() < 1e-6);
        assert_eq!(tool.form, Some((3, "soft-shape".to_owned())));
        assert_eq!(tool.texture.as_ref().map(|t| t.0.as_str()), Some("paper-01"));
        assert!((tool.blend - 0.5).abs() < 1e-6 && (tool.dilution - 0.4).abs() < 1e-6);
        assert!(tool.pressure_size && tool.pressure_density && !tool.pressure_blend && tool.pressure_dilution);
    }

    #[test]
    fn something_else_is_not_a_tool() {
        assert!(read(b"a plain text file\n").is_none());
        assert!(read(&[0xff, 0x00, 0x13]).is_none());
    }
}
