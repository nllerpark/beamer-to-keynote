mod pdfium_engine;
mod process_environment;
mod tex_engine;

use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
    process::Command,
    thread,
    time::Duration,
    time::{SystemTime, UNIX_EPOCH},
};
use tauri::{AppHandle, Manager};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct EnvironmentStatus {
    keynote_installed: bool,
    native_engine_ready: bool,
    tex_engine_ready: bool,
    tex_engine_bundled: bool,
    tex_engine_paths: Vec<String>,
    missing_tex_tools: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TexInfo {
    name: String,
    fonts: Vec<String>,
    missing_fonts: Vec<String>,
    keynote_installed: bool,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EmbeddedFontInfo {
    pub(crate) name: String,
    pub(crate) subset: bool,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PdfInfo {
    pub(crate) name: String,
    pub(crate) pages: usize,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) file_size: u64,
    pub(crate) text_items: usize,
    pub(crate) fonts: Vec<String>,
    pub(crate) missing_fonts: Vec<String>,
    pub(crate) extractable_fonts: Vec<EmbeddedFontInfo>,
    pub(crate) keynote_installed: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ManifestResult {
    pub(crate) manifest_path: String,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) pages: usize,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ConvertRequest {
    input_path: String,
    output_directory: Option<String>,
    open_after: bool,
    #[serde(default)]
    mode: ConvertMode,
    #[serde(default)]
    extract_embedded_fonts: bool,
}

#[derive(Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
enum ConvertMode {
    #[default]
    Editable,
    ExactImage,
    TexDirect,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ConvertResult {
    output_path: String,
    pages: usize,
    extracted_fonts: usize,
    skipped_fonts: Vec<String>,
}

fn keynote_path() -> Option<&'static str> {
    [
        "/Applications/Keynote.app",
        "/Applications/Keynote Creator Studio.app",
    ]
    .into_iter()
    .find(|path| Path::new(path).is_dir())
}

fn pdfium_library_path(app: &AppHandle) -> Result<PathBuf, String> {
    let resource_directory = app
        .path()
        .resource_dir()
        .map_err(|error| error.to_string())?;
    let bundled_framework = resource_directory
        .parent()
        .map(|contents| contents.join("Frameworks/libpdfium.dylib"));
    let legacy_resource = resource_directory.join("pdfium/lib/libpdfium.dylib");
    let development = pdfium_engine::development_library_path();
    bundled_framework
        .into_iter()
        .chain([legacy_resource, development])
        .find(|path| path.is_file())
        .ok_or("PDFium 네이티브 엔진이 앱에 포함되지 않았습니다.".into())
}

#[tauri::command]
fn environment_status(app: AppHandle) -> EnvironmentStatus {
    // dvisvgm may be bundled (GPL build); the TeX programs never are.
    let tex_tools = ["xelatex", "dvilualatex", "dvisvgm", "kpsewhich"];
    let resolved_tex_tools = tex_tools
        .into_iter()
        .map(|name| (name, tool_path(&app, name).ok()))
        .collect::<Vec<_>>();
    let bundled_renderer = bundled_tool_path(&app, "dvisvgm").is_some();
    let mut tex_engine_paths = resolved_tex_tools
        .iter()
        .filter_map(|(_, path)| path.as_ref())
        .map(|path| path.to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    let mut missing_tex_tools = resolved_tex_tools
        .iter()
        .filter(|(_, path)| path.is_none())
        .map(|(name, _)| (*name).to_owned())
        .collect::<Vec<_>>();
    let ghostscript = bundled_ghostscript_library(&app)
        .or_else(process_environment::ghostscript_library_path);
    let bundled_ghostscript = bundled_ghostscript_library(&app).is_some();
    match ghostscript {
        Some(path) => tex_engine_paths.push(path.to_string_lossy().into_owned()),
        // Ghostscript is only consulted for EPS/PostScript specials, so a
        // missing libgs is reported without blocking conversion outright.
        None => missing_tex_tools.push("libgs (brew install ghostscript)".into()),
    }

    let blocking_tools = missing_tex_tools
        .iter()
        .filter(|tool| !tool.starts_with("libgs"))
        .count();

    EnvironmentStatus {
        keynote_installed: keynote_path().is_some(),
        native_engine_ready: pdfium_library_path(&app).is_ok_and(|path| path.is_file()),
        tex_engine_ready: blocking_tools == 0,
        tex_engine_bundled: bundled_renderer && bundled_ghostscript,
        tex_engine_paths,
        missing_tex_tools,
    }
}

fn apple_script_string(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

#[tauri::command]
fn choose_source(prompt: String) -> Result<Option<String>, String> {
    let script = format!(
        "set f to choose file with prompt \"{}\"\nPOSIX path of f",
        apple_script_string(&prompt)
    );
    let output = Command::new("/usr/bin/osascript")
        .args(["-e", &script])
        .output()
        .map_err(|error| error.to_string())?;
    if output.status.success() {
        Ok(Some(
            String::from_utf8_lossy(&output.stdout).trim().to_string(),
        ))
    } else if output.status.code() == Some(1)
        && String::from_utf8_lossy(&output.stderr).contains("-128")
    {
        Ok(None)
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

#[tauri::command]
fn choose_output_folder(prompt: String) -> Result<Option<String>, String> {
    let script = format!(
        "set f to choose folder with prompt \"{}\"\nPOSIX path of f",
        apple_script_string(&prompt)
    );
    let output = Command::new("/usr/bin/osascript")
        .args(["-e", &script])
        .output()
        .map_err(|error| error.to_string())?;
    if output.status.success() {
        Ok(Some(
            String::from_utf8_lossy(&output.stdout).trim().to_string(),
        ))
    } else if output.status.code() == Some(1)
        && String::from_utf8_lossy(&output.stderr).contains("-128")
    {
        Ok(None)
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

fn installed_font_names() -> Result<BTreeSet<String>, String> {
    let script = r#"ObjC.import("AppKit");
const manager = $.NSFontManager.sharedFontManager;
const names = [];
for (const fonts of [manager.availableFonts, manager.availableFontFamilies]) {
  for (let index = 0; index < fonts.count; index++) {
    names.push(ObjC.unwrap(fonts.objectAtIndex(index)));
  }
}
names.join("\n");"#;
    let output = Command::new("/usr/bin/osascript")
        .args(["-l", "JavaScript", "-e", script])
        .output()
        .map_err(|error| format!("macOS 서체 목록을 읽지 못했습니다: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "macOS 서체 목록을 읽지 못했습니다: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(str::to_owned)
        .collect())
}

fn strip_tex_comments(source: &str) -> String {
    source
        .lines()
        .map(|line| {
            let mut escaped = false;
            for (index, character) in line.char_indices() {
                if character == '%' && !escaped {
                    return &line[..index];
                }
                if character == '\\' {
                    escaped = !escaped;
                } else {
                    escaped = false;
                }
            }
            line
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn skip_tex_space(source: &str, mut cursor: usize) -> usize {
    while source
        .as_bytes()
        .get(cursor)
        .is_some_and(|byte| byte.is_ascii_whitespace())
    {
        cursor += 1;
    }
    cursor
}

fn tex_group(source: &str, start: usize, open: u8, close: u8) -> Option<(&str, usize)> {
    if source.as_bytes().get(start).copied()? != open {
        return None;
    }
    let bytes = source.as_bytes();
    let mut depth = 0usize;
    let mut escaped = false;
    for (index, byte) in bytes.iter().copied().enumerate().skip(start) {
        if escaped {
            escaped = false;
            continue;
        }
        if byte == b'\\' {
            escaped = true;
            continue;
        }
        if byte == open {
            depth += 1;
        } else if byte == close {
            depth -= 1;
            if depth == 0 {
                return Some((&source[start + 1..index], index + 1));
            }
        }
    }
    None
}

fn tex_font_argument(source: &str, after_command: usize, declares_macro: bool) -> Option<String> {
    let mut cursor = skip_tex_space(source, after_command);

    if declares_macro {
        if source.as_bytes().get(cursor) == Some(&b'\\') {
            cursor += 1;
            while source
                .as_bytes()
                .get(cursor)
                .is_some_and(|byte| byte.is_ascii_alphabetic() || *byte == b'@')
            {
                cursor += 1;
            }
        } else if let Some((macro_name, next)) = tex_group(source, cursor, b'{', b'}')
            && macro_name.trim_start().starts_with('\\')
        {
            cursor = next;
        }
        cursor = skip_tex_space(source, cursor);
    }

    while source.as_bytes().get(cursor) == Some(&b'[') {
        let (_, next) = tex_group(source, cursor, b'[', b']')?;
        cursor = skip_tex_space(source, next);
    }

    let (font, _) = tex_group(source, cursor, b'{', b'}')?;
    let font = font.trim().trim_matches('"');
    if font.is_empty() || font.contains('\\') || font.contains('#') {
        return None;
    }
    Some(font.to_owned())
}

fn explicit_tex_fonts(source: &str) -> BTreeSet<String> {
    const COMMANDS: [(&str, bool); 13] = [
        ("setmainfont", false),
        ("setsansfont", false),
        ("setmonofont", false),
        ("setmathfont", false),
        ("fontspec", false),
        ("newfontfamily", true),
        ("newfontface", true),
        ("setCJKmainfont", false),
        ("setCJKsansfont", false),
        ("setCJKmonofont", false),
        ("newCJKfontfamily", true),
        ("setmainhangulfont", false),
        ("setsanshangulfont", false),
    ];

    let source = strip_tex_comments(source);
    let mut fonts = BTreeSet::new();
    for (command, declares_macro) in COMMANDS {
        let needle = format!("\\{command}");
        for (start, _) in source.match_indices(&needle) {
            let after = start + needle.len();
            if source
                .as_bytes()
                .get(after)
                .is_some_and(|byte| byte.is_ascii_alphabetic() || *byte == b'@')
            {
                continue;
            }
            if let Some(font) = tex_font_argument(&source, after, declares_macro) {
                fonts.insert(font);
            }
        }
    }
    fonts
}

fn normalized_font_name(font: &str) -> String {
    font.chars()
        .filter(|character| character.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn tex_font_is_available(font: &str, installed_fonts: &BTreeSet<String>) -> bool {
    let requested = normalized_font_name(font);
    installed_fonts.iter().any(|installed| {
        let installed = normalized_font_name(installed);
        installed == requested
            || installed
                .strip_suffix("regular")
                .is_some_and(|family| family == requested)
    })
}

#[tauri::command]
async fn inspect_tex(path: String) -> Result<TexInfo, String> {
    tauri::async_runtime::spawn_blocking(move || {
        if validate_source(&path)? != "tex" {
            return Err("올바른 Beamer .tex 파일을 선택해 주세요.".into());
        }
        let source_path = Path::new(&path);
        let metadata = fs::metadata(source_path).map_err(|error| error.to_string())?;
        if metadata.len() > 8 * 1024 * 1024 {
            return Err("TeX 원본이 사전 점검 제한인 8 MiB를 초과합니다.".into());
        }
        let source = fs::read_to_string(source_path)
            .map_err(|error| format!("TeX 원본을 읽지 못했습니다: {error}"))?;
        let fonts = explicit_tex_fonts(&source);
        let installed = installed_font_names()?;
        let missing_fonts = fonts
            .iter()
            .filter(|font| !tex_font_is_available(font, &installed))
            .cloned()
            .collect();
        Ok(TexInfo {
            name: source_path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned(),
            fonts: fonts.into_iter().collect(),
            missing_fonts,
            keynote_installed: keynote_path().is_some(),
        })
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
async fn inspect_pdf(app: AppHandle, path: String) -> Result<PdfInfo, String> {
    tauri::async_runtime::spawn_blocking(move || {
        validate_pdf(&path)?;
        let library = pdfium_library_path(&app)?;
        let installed_fonts = installed_font_names()?;
        pdfium_engine::inspect(
            Path::new(&path),
            &library,
            keynote_path().is_some(),
            &installed_fonts,
        )
    })
    .await
    .map_err(|error| error.to_string())?
}

fn source_extension(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase)
}

fn validate_source(path: &str) -> Result<&'static str, String> {
    let source = Path::new(path);
    if !source.is_file() {
        return Err("올바른 입력 파일을 선택해 주세요.".into());
    }
    match source_extension(source).as_deref() {
        Some("pdf") => Ok("pdf"),
        Some("tex") => Ok("tex"),
        _ => Err("PDF 또는 Beamer .tex 파일을 선택해 주세요.".into()),
    }
}

fn validate_pdf(path: &str) -> Result<(), String> {
    let source = Path::new(path);
    if !source.is_file()
        || source
            .extension()
            .is_none_or(|ext| !ext.eq_ignore_ascii_case("pdf"))
    {
        return Err("올바른 PDF 파일을 선택해 주세요.".into());
    }
    Ok(())
}

fn unique_output_path(source: &Path, output_dir: Option<&str>, mode: ConvertMode) -> PathBuf {
    let directory = output_dir.map(PathBuf::from).unwrap_or_else(|| {
        source
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf()
    });
    let stem = source.file_stem().unwrap_or_default().to_string_lossy();
    let suffix = match mode {
        ConvertMode::Editable => "editable",
        ConvertMode::ExactImage => "exact",
        ConvertMode::TexDirect => "direct",
    };
    let base = directory.join(format!("{stem}-{suffix}.key"));
    if !base.exists() {
        return base;
    }
    (2..10_000)
        .map(|index| directory.join(format!("{stem}-{suffix}-{index}.key")))
        .find(|path| !path.exists())
        .unwrap_or(base)
}

/// Executables shipped inside the GPL build. Absent from the BSD build, which
/// redistributes no GPL-licensed programs and uses the system copies instead.
fn bundled_tool_path(app: &AppHandle, name: &str) -> Option<PathBuf> {
    let resource_directory = app.path().resource_dir().ok()?;
    let contents = resource_directory.parent()?;
    [
        contents.join("MacOS").join(name),
        resource_directory.join(name),
    ]
    .into_iter()
    .find(|path| path.is_file())
}

/// Prefers a bundled copy over the system one, so the GPL build is insulated
/// from whatever dvisvgm the host happens to provide.
fn tool_path(app: &AppHandle, name: &str) -> Result<PathBuf, String> {
    if let Some(bundled) = bundled_tool_path(app, name) {
        return Ok(bundled);
    }
    system_tool_path(name)
}

fn system_tool_path(name: &str) -> Result<PathBuf, String> {
    process_environment::system_tool_candidates(name)
        .find(|path| path.is_file())
        .ok_or_else(|| {
            format!(
                "{name} 직접 렌더링 엔진을 찾지 못했습니다. MacTeX를 설치하고 /Library/TeX/texbin이 PATH에서 보이도록 설정한 뒤 TeXKey를 다시 실행해 주세요."
            )
        })
}

/// Root of the host TeX installation, used to point a relocated dvisvgm at
/// texmf.cnf and the font maps it would otherwise locate from its own path.
fn texmf_root_path(kpsewhich: &Path) -> Option<PathBuf> {
    let mut command = Command::new(kpsewhich);
    process_environment::configure_tex_process(&mut command, Path::new("/"));
    let output = command.args(["-var-value=SELFAUTOPARENT"]).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let root = PathBuf::from(String::from_utf8_lossy(&output.stdout).trim());
    root.is_dir().then_some(root)
}

/// Ghostscript shared library vendored into the GPL build, alongside its
/// relocated dependency closure.
fn bundled_ghostscript_library(app: &AppHandle) -> Option<PathBuf> {
    let library = app
        .path()
        .resource_dir()
        .ok()?
        .join("ghostscript/lib/libgs.dylib");
    library.is_file().then_some(library)
}

fn install_bundled_fonts(app: &AppHandle, resource_dir: &Path) -> Result<usize, String> {
    let bundled_fonts = resource_dir.join("fonts");
    if !bundled_fonts.is_dir() {
        return Ok(0);
    }
    let font_dir = app.path().font_dir().map_err(|error| error.to_string())?;
    fs::create_dir_all(&font_dir).map_err(|error| error.to_string())?;
    let mut installed = 0;
    for entry in fs::read_dir(&bundled_fonts).map_err(|error| error.to_string())? {
        let source = entry.map_err(|error| error.to_string())?.path();
        let is_font = source.extension().is_some_and(|value| {
            value.eq_ignore_ascii_case("otf") || value.eq_ignore_ascii_case("ttf")
        });
        if !is_font {
            continue;
        }
        let destination = font_dir.join(
            source
                .file_name()
                .ok_or("앱에 포함된 서체 파일명이 올바르지 않습니다.")?,
        );
        let should_copy = match (fs::metadata(&source), fs::metadata(&destination)) {
            (Ok(source_meta), Ok(destination_meta)) => source_meta.len() != destination_meta.len(),
            (Ok(_), Err(_)) => true,
            _ => false,
        };
        if should_copy {
            fs::copy(&source, &destination)
                .map_err(|error| format!("편집용 서체를 설치하지 못했습니다: {error}"))?;
            installed += 1;
        }
    }
    if installed > 0 {
        thread::sleep(Duration::from_millis(800));
    }
    Ok(installed)
}

#[tauri::command]
async fn convert_pdf(app: AppHandle, request: ConvertRequest) -> Result<ConvertResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let source_kind = validate_source(&request.input_path)?;
        let keynote = keynote_path().ok_or("Keynote 앱이 설치되어 있지 않습니다.")?;
        let source = PathBuf::from(&request.input_path);
        let mode = if source_kind == "tex" {
            ConvertMode::TexDirect
        } else {
            if matches!(request.mode, ConvertMode::TexDirect) {
                return Err("PDF에는 PDF 변환 방식을 선택해 주세요.".into());
            }
            request.mode
        };
        let output = unique_output_path(&source, request.output_directory.as_deref(), mode);
        if let Some(parent) = output.parent() {
            fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }

        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| error.to_string())?
            .as_millis();
        let work = std::env::temp_dir().join(format!("texkey-{stamp}"));
        fs::create_dir_all(&work).map_err(|error| error.to_string())?;
        let resource_dir = app
            .path()
            .resource_dir()
            .map_err(|error| error.to_string())?;
        let mut extracted_fonts = 0;
        let mut skipped_fonts = Vec::new();
        let manifest = match mode {
            ConvertMode::Editable => {
                let library = pdfium_library_path(&app)?;
                install_bundled_fonts(&app, &resource_dir)?;
                let mut preserved_pdf_font_names = BTreeSet::new();
                if request.extract_embedded_fonts {
                    let font_dir = app.path().font_dir().map_err(|error| error.to_string())?;
                    let report = pdfium_engine::extract_embedded_fonts(
                        &source,
                        &library,
                        &font_dir,
                        &installed_font_names()?,
                    )?;
                    extracted_fonts = report.installed;
                    skipped_fonts = report.skipped;
                    preserved_pdf_font_names = report.preserved_pdf_font_names;
                    if extracted_fonts > 0 {
                        thread::sleep(Duration::from_millis(900));
                    }
                }
                pdfium_engine::manifest(&source, &work, &library, &preserved_pdf_font_names)?
            }
            ConvertMode::ExactImage => {
                let library = pdfium_library_path(&app)?;
                pdfium_engine::exact_image_manifest(&source, &work, &library)?
            }
            ConvertMode::TexDirect => {
                let home_directory = app.path().home_dir().map_err(|error| error.to_string())?;
                // xelatex/dvilualatex always come from MacTeX: the GPL build
                // bundles a renderer, not a TeX distribution.
                let xelatex = system_tool_path("xelatex")?;
                let dvilualatex = system_tool_path("dvilualatex")?;
                let dvisvgm = tool_path(&app, "dvisvgm")?;
                // Optional: only EPS/PostScript specials need it.
                let ghostscript_library = bundled_ghostscript_library(&app)
                    .or_else(process_environment::ghostscript_library_path);
                // A bundled dvisvgm sits outside the TeX tree, so kpathsea
                // cannot locate texmf.cnf from its own path and would fall
                // back to running Metafont for every font.
                let texmf_root = bundled_tool_path(&app, "dvisvgm")
                    .and_then(|_| system_tool_path("kpsewhich").ok())
                    .and_then(|kpsewhich| texmf_root_path(&kpsewhich));
                tex_engine::manifest(
                    &source,
                    &work,
                    tex_engine::Runtime {
                        xelatex: &xelatex,
                        dvilualatex: &dvilualatex,
                        dvisvgm: &dvisvgm,
                        ghostscript_library: ghostscript_library.as_deref(),
                        texmf_root: texmf_root.as_deref(),
                        home_directory: &home_directory,
                    },
                )?
            }
        };

        let _ = Command::new("/usr/bin/open")
            .args(["-g", "-a", keynote])
            .status();
        let script_path = [
            resource_dir.join("keynote_editable_import.applescript"),
            resource_dir.join("_up_/keynote_editable_import.applescript"),
        ]
        .into_iter()
        .find(|path| path.is_file())
        .ok_or("Keynote 자동화 스크립트가 앱에 포함되지 않았습니다.")?;
        let result = Command::new("/usr/bin/osascript")
            .arg(script_path)
            .arg(&output)
            .arg(manifest.width.to_string())
            .arg(manifest.height.to_string())
            .arg(&manifest.manifest_path)
            .arg(if request.open_after { "true" } else { "false" })
            .output()
            .map_err(|error| error.to_string())?;
        if !result.status.success() {
            return Err(format!(
                "Keynote 문서를 만들지 못했습니다: {}",
                String::from_utf8_lossy(&result.stderr).trim()
            ));
        }
        if !output.exists() {
            return Err("Keynote가 출력 파일을 만들지 않았습니다.".into());
        }
        let _ = fs::remove_dir_all(&work);
        Ok(ConvertResult {
            output_path: output.to_string_lossy().to_string(),
            pages: manifest.pages,
            extracted_fonts,
            skipped_fonts,
        })
    })
    .await
    .map_err(|error| error.to_string())?
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            environment_status,
            choose_source,
            choose_output_folder,
            inspect_tex,
            inspect_pdf,
            convert_pdf
        ])
        .run(tauri::generate_context!())
        .expect("error while running TeXKey");
}

#[cfg(test)]
mod tests {
    use super::{explicit_tex_fonts, tex_font_is_available};
    use std::collections::BTreeSet;

    #[test]
    fn finds_explicit_fontspec_families() {
        let source = r#"
          % \setmainfont{Commented Font}
          \setmainfont[Ligatures=TeX]{Iowan Old Style}
          \setsansfont{Avenir Next}
          \newfontfamily\codeface[Scale=MatchLowercase]{JetBrains Mono}
          \setCJKmainfont{Noto Serif CJK KR}
          \fontspec{\dynamicfont}
        "#;
        assert_eq!(
            explicit_tex_fonts(source),
            BTreeSet::from([
                "Avenir Next".to_owned(),
                "Iowan Old Style".to_owned(),
                "JetBrains Mono".to_owned(),
                "Noto Serif CJK KR".to_owned(),
            ])
        );
    }

    #[test]
    fn matches_family_and_regular_postscript_names() {
        let installed = BTreeSet::from([
            "Avenir Next".to_owned(),
            "LibertinusSerif-Regular".to_owned(),
        ]);
        assert!(tex_font_is_available("Avenir Next", &installed));
        assert!(tex_font_is_available("Libertinus Serif", &installed));
        assert!(!tex_font_is_available(
            "Uninstalled Scholarly Face",
            &installed
        ));
    }
}
