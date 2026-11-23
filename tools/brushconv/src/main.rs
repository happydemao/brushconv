//! `brushconv`: convert brush files between ProCraft, Photoshop, Krita, GIMP, Infinite Painter,
//! SAI2 and PhotoCraft.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use brushconv::{BrushSet, OutputFormat, read_path, write_set};

const HELP: &str = "\
brushconv — cross-platform brush format converter

Reads ProCraft .brush/.brushset/.brushlibrary, Photoshop .abr, Krita .kpp/.bundle,
GIMP .gbr/.gih/.vbr, Clip Studio .sut/.sutg, Infinite Painter .prbr, PaintTool SAI2
.saitdat and PhotoCraft .pcbrushes, and writes ProCraft, Photoshop, Krita, GIMP,
Infinite Painter, SAI2 or PhotoCraft brushes (Clip Studio .sut is read-only;
Infinite Painter's .prbr and SAI2's .saitdat are new: check them in the app before
relying on them; a PhotoCraft group names a computed tip, not a sampled one).

USAGE:
    brushconv [OPTIONS] <INPUT>...

ARGS:
    <INPUT>...             Brush files, or folders of them

OPTIONS:
    -o, --output <PATH>    Output file, or folder for several brushes/inputs
    -t, --to <FORMAT>      brush | brushset | abr | kpp | bundle | gbr | prbr | saitdat | pcbrushes | json
    -n, --name <NAME>      Name the output set (default: the input's name)
    -l, --list             List the brushes and their settings, writing nothing
    -r, --recursive        Look in folders' subfolders too
        --flat             When a folder holds several sets, write their files side by side
    -h, --help             Show this help
    -V, --version          Show the version

Without --to, the output's ending picks the format; without either, a set of several
brushes becomes a .brushset and a single brush a .brush.

EXAMPLES:
    brushconv Soft.abr -o Soft.brush
    brushconv MySet.brushset --to abr -o MySet.abr
    brushconv Watercolour.brushset --to kpp -o krita-watercolour/
    brushconv Inking.brushset --to bundle
    brushconv *.gbr --to brushset -o GimpBrushes.brushset
";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let options = match Options::parse(&args) {
        Ok(Command::Run(options)) => options,
        Ok(Command::Help) => {
            let _ = write!(std::io::stdout(), "{HELP}");
            return ExitCode::SUCCESS;
        }
        Ok(Command::Version) => {
            say(&format!("brushconv {}", env!("CARGO_PKG_VERSION")));
            return ExitCode::SUCCESS;
        }
        Err(message) => {
            eprintln!("brushconv: {message}");
            eprintln!("Try `brushconv --help`.");
            return ExitCode::from(2);
        }
    };
    match run(&options) {
        Ok(()) => ExitCode::SUCCESS,
        Err(failures) => {
            for failure in &failures {
                eprintln!("brushconv: {failure}");
            }
            ExitCode::FAILURE
        }
    }
}

#[derive(Debug)]
struct Options {
    inputs: Vec<PathBuf>,
    output: Option<PathBuf>,
    to: Option<OutputFormat>,
    name: Option<String>,
    list: bool,
    recursive: bool,
    flat: bool,
}

enum Command {
    Help,
    Version,
    Run(Options),
}

impl Options {
    fn parse(args: &[String]) -> Result<Command, String> {
        let mut options = Options {
            inputs: Vec::new(),
            output: None,
            to: None,
            name: None,
            list: false,
            recursive: false,
            flat: false,
        };
        let mut rest = args.iter();
        while let Some(arg) = rest.next() {
            match arg.as_str() {
                "-h" | "--help" => return Ok(Command::Help),
                "-V" | "--version" => return Ok(Command::Version),
                "-o" | "--output" => {
                    options.output = Some(PathBuf::from(rest.next().ok_or("--output needs a path")?));
                }
                "-t" | "--to" => {
                    let name = rest.next().ok_or("--to needs a format")?;
                    options.to =
                        Some(OutputFormat::from_name(name).ok_or_else(|| format!("no format called `{name}`"))?);
                }
                "-n" | "--name" => options.name = Some(rest.next().ok_or("--name needs a name")?.clone()),
                "-l" | "--list" => options.list = true,
                "-r" | "--recursive" => options.recursive = true,
                "--flat" => options.flat = true,
                "--" => {
                    for arg in rest.by_ref() {
                        options.inputs.push(PathBuf::from(arg));
                    }
                }
                arg if arg.starts_with('-') && arg != "-" => {
                    return Err(format!("unknown option `{arg}`"));
                }
                arg => options.inputs.push(PathBuf::from(arg)),
            }
        }
        if options.inputs.is_empty() {
            return Err("no brush files given".to_owned());
        }
        Ok(Command::Run(options))
    }
}

fn run(options: &Options) -> Result<(), Vec<String>> {
    let files = match collect_files(&options.inputs, options.recursive) {
        Ok(files) => files,
        Err(e) => return Err(vec![e]),
    };
    if files.is_empty() {
        return Err(vec!["no brush files found".to_owned()]);
    }
    if options.list {
        let mut failures = Vec::new();
        for file in &files {
            match read_path(file) {
                Ok(set) => print_list(&set),
                Err(e) => failures.push(format!("{}: {e}", file.display())),
            }
        }
        return if failures.is_empty() { Ok(()) } else { Err(failures) };
    }
    let mut failures = Vec::new();
    let mut sets = Vec::new();
    for file in &files {
        match read_path(file) {
            Ok(set) => sets.push(set),
            Err(e) => failures.push(format!("{}: {e}", file.display())),
        }
    }
    if sets.is_empty() {
        return Err(failures);
    }
    let total: usize = sets.iter().map(|set| set.brushes.len()).sum();
    let format = options.to.or_else(|| output_format(options)).unwrap_or(if total > 1 {
        OutputFormat::BrushSet
    } else {
        OutputFormat::Brush
    });
    if sets.len() > 1 && format.merges_inputs() {
        match write_merged(options, &sets, format) {
            Ok(()) => {}
            Err(e) => failures.push(e),
        }
        return if failures.is_empty() { Ok(()) } else { Err(failures) };
    }
    if sets.len() > 1 && format.one_per_brush() && options.output.as_deref().is_some_and(looks_like_file) {
        return Err(vec![format!(
            "several inputs need an output folder, not a single file {}",
            options.output.as_deref().map(|p| p.display().to_string()).unwrap_or_default()
        )]);
    }
    for set in &sets {
        match convert_one(options, set, format, sets.len() > 1) {
            Ok(()) => {}
            Err(e) => failures.push(e),
        }
    }
    if failures.is_empty() { Ok(()) } else { Err(failures) }
}

/// The format `--output`'s extension names, if it names one.
fn output_format(options: &Options) -> Option<OutputFormat> {
    options.output.as_deref().and_then(Path::extension).and_then(|e| OutputFormat::from_extension(&e.to_string_lossy()))
}

/// Several inputs' brushes in one file (`.brushset`, `.abr`, `.bundle`, JSON).
fn write_merged(options: &Options, sets: &[BrushSet], format: OutputFormat) -> Result<(), String> {
    let mut merged = sets.first().cloned().unwrap_or(BrushSet {
        name: "Brushes".to_owned(),
        brushes: Vec::new(),
        source: brushconv::SourceFormat::Unknown,
        file_name: String::new(),
    });
    merged.brushes.clear();
    for set in sets {
        merged.brushes.extend(set.brushes.iter().cloned());
    }
    if let Some(name) = &options.name {
        merged.name.clone_from(name);
    }
    let written =
        write_set(&merged, format, options.output.as_deref(), options.name.as_deref()).map_err(|e| e.to_string())?;
    for path in written {
        say(&path.display().to_string());
    }
    Ok(())
}

fn convert_one(options: &Options, set: &BrushSet, format: OutputFormat, many: bool) -> Result<(), String> {
    let out = output_for(options, set, format, many)?;
    let written = write_set(set, format, out.as_deref(), options.name.as_deref())
        .map_err(|e| format!("{}: {e}", set.file_name))?;
    for path in written {
        say(&path.display().to_string());
    }
    Ok(())
}

/// Where this input's output goes: as `--output` says for one input, else a folder of its own.
fn output_for(options: &Options, set: &BrushSet, format: OutputFormat, many: bool) -> Result<Option<PathBuf>, String> {
    let Some(output) = &options.output else { return Ok(None) };
    if !many || !format.one_per_brush() {
        return Ok(Some(output.clone()));
    }
    if !options.flat && set.brushes.len() > 1 {
        let stem = Path::new(&set.file_name)
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| set.name.clone());
        Ok(Some(output.join(stem)))
    } else {
        Ok(Some(output.clone()))
    }
}

/// Every brush file among `inputs`, folders gone through.
fn collect_files(inputs: &[PathBuf], recursive: bool) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();
    for input in inputs {
        if input.is_dir() {
            walk(input, recursive, &mut files).map_err(|e| format!("{}: {e}", input.display()))?;
        } else if input.is_file() {
            files.push(input.clone());
        } else {
            return Err(format!("{}: no such file or folder", input.display()));
        }
    }
    files.sort();
    files.dedup();
    Ok(files)
}

fn walk(dir: &Path, recursive: bool, files: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            if recursive {
                walk(&path, recursive, files)?;
            }
        } else if is_brush_file(&path) {
            files.push(path);
        }
    }
    Ok(())
}

/// Whether `path` names a file (or would, if it existed) rather than a folder.
fn looks_like_file(path: &Path) -> bool {
    path.is_file() || (!path.is_dir() && path.extension().is_some_and(|e| !e.is_empty()))
}

fn is_brush_file(path: &Path) -> bool {
    const EXTENSIONS: [&str; 15] = [
        "brush",
        "brushset",
        "brushlibrary",
        "abr",
        "gbr",
        "gih",
        "vbr",
        "kpp",
        "bundle",
        "sut",
        "sutg",
        "prbr",
        "saitdat",
        "pcbrushes",
        "pat",
    ];
    path.extension().map(|e| e.to_string_lossy().to_ascii_lowercase()).is_some_and(|e| EXTENSIONS.contains(&e.as_str()))
}

/// Print a line, ignoring a closed pipe (`brushconv … | head`).
fn say(line: &str) {
    let _ = writeln!(std::io::stdout(), "{line}");
}

fn print_list(set: &BrushSet) {
    say(&format!(
        "{}  {}  {} brush{}",
        set.file_name,
        set.source.label(),
        set.brushes.len(),
        plural(set.brushes.len())
    ));
    for (index, brush) in set.brushes.iter().enumerate() {
        say(&format!(
            "  {:>2}. {:<32} size {:.0}–{:.0}  spacing {:.0}%  opacity {:.0}%",
            index + 1,
            brush.name,
            brush.min_size,
            brush.max_size,
            brush.spacing * 100.0,
            brush.max_opacity * 100.0
        ));
    }
}

fn plural(count: usize) -> &'static str {
    if count == 1 { "" } else { "es" }
}
