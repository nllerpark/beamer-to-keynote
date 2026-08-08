use crate::{ManifestResult, process_environment::configure_tex_process};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use std::{
    fs,
    os::unix::fs::symlink,
    path::{Path, PathBuf},
    process::{Command, Output},
};

fn command_error(label: &str, output: &Output) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let details = if stderr.trim().is_empty() {
        stdout.trim()
    } else {
        stderr.trim()
    };

    if let Some(line) = details.lines().find(|line| line.contains("XML error:")) {
        return format!("{label}에 실패했습니다: {line}");
    }
    if let Some(line) = details
        .lines()
        .find(|line| line.contains("No such file") || line.contains(" not found:"))
    {
        return format!("{label}에 필요한 외부 파일이 없습니다: {line}");
    }
    let tail = details.lines().rev().take(18).collect::<Vec<_>>();
    format!(
        "{label}에 실패했습니다:\n{}",
        tail.into_iter().rev().collect::<Vec<_>>().join("\n")
    )
}

fn attribute_number(svg: &str, name: &str) -> Result<f64, String> {
    let marker = format!("{name}='");
    let start = svg
        .find(&marker)
        .ok_or_else(|| format!("SVG에서 {name}를 찾지 못했습니다."))?
        + marker.len();
    let value = svg[start..]
        .split_once('\'')
        .map(|(value, _)| value)
        .ok_or_else(|| format!("SVG의 {name} 값이 손상되었습니다."))?;
    let numeric = value.trim_end_matches("pt");
    numeric
        .parse::<f64>()
        .map_err(|error| format!("SVG의 {name} 값을 읽지 못했습니다: {error}"))
}

fn svg_files(directory: &Path) -> Result<Vec<PathBuf>, String> {
    let mut slides = fs::read_dir(directory)
        .map_err(|error| error.to_string())?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|value| value.to_str())
                .is_some_and(|name| name.starts_with("slide-") && name.ends_with(".svg"))
        })
        .collect::<Vec<_>>();
    slides.sort_by_key(|path| {
        path.file_stem()
            .and_then(|value| value.to_str())
            .and_then(|value| value.strip_prefix("slide-"))
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(usize::MAX)
    });
    if slides.is_empty() {
        return Err("TeX 렌더러가 슬라이드를 만들지 않았습니다.".into());
    }
    Ok(slides)
}

fn bitmap_mime_type(path: &Path) -> Option<&'static str> {
    match path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("jpg" | "jpeg") => Some("image/jpeg"),
        Some("png") => Some("image/png"),
        Some("gif") => Some("image/gif"),
        Some("webp") => Some("image/webp"),
        _ => None,
    }
}

fn embed_linked_bitmaps(svg_path: &Path) -> Result<(), String> {
    let directory = svg_path.parent().unwrap_or_else(|| Path::new("."));
    let mut svg = fs::read_to_string(svg_path).map_err(|error| error.to_string())?;
    let mut changed = false;

    for marker in ["xlink:href='", "href='", "xlink:href=\"", "href=\""] {
        let quote = marker
            .chars()
            .last()
            .ok_or("SVG 이미지 속성 표시가 올바르지 않습니다.")?;
        let mut search_from = 0;
        while let Some(relative_start) = svg[search_from..].find(marker) {
            let value_start = search_from + relative_start + marker.len();
            let Some(relative_end) = svg[value_start..].find(quote) else {
                break;
            };
            let value_end = value_start + relative_end;
            let value = svg[value_start..value_end].to_owned();
            let should_skip = value.starts_with('#')
                || value.starts_with("data:")
                || value.starts_with("http:")
                || value.starts_with("https:")
                || value.starts_with("file:")
                || value.starts_with("//");
            if should_skip {
                search_from = value_end + quote.len_utf8();
                continue;
            }

            let asset = directory.join(&value);
            let Some(mime_type) = bitmap_mime_type(&asset) else {
                search_from = value_end + quote.len_utf8();
                continue;
            };
            if !asset.is_file() {
                return Err(format!(
                    "SVG가 참조하는 이미지 파일을 찾지 못했습니다: {}",
                    asset.display()
                ));
            }
            let bytes = fs::read(&asset)
                .map_err(|error| format!("SVG 이미지 자산을 읽지 못했습니다: {error}"))?;
            let data_uri = format!("data:{mime_type};base64,{}", BASE64.encode(bytes));
            svg.replace_range(value_start..value_end, &data_uri);
            search_from = value_start + data_uri.len() + quote.len_utf8();
            changed = true;
        }
    }

    if changed {
        fs::write(svg_path, svg).map_err(|error| error.to_string())?;
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TexEngine {
    XeLatex,
    LuaLatex,
}

impl TexEngine {
    fn output_extension(self) -> &'static str {
        match self {
            Self::XeLatex => "xdv",
            Self::LuaLatex => "dvi",
        }
    }

    fn display_name(self) -> &'static str {
        match self {
            Self::XeLatex => "XeLaTeX",
            Self::LuaLatex => "LuaLaTeX",
        }
    }
}

fn preferred_engine_for_source(source: &str) -> TexEngine {
    let leading_source = source.lines().take(80).collect::<Vec<_>>().join("\n");
    let lowercase = leading_source.to_ascii_lowercase();

    for line in lowercase.lines() {
        let trimmed = line.trim_start();
        let is_magic_comment = trimmed.starts_with("%!")
            || trimmed.starts_with("% !")
            || trimmed.contains("tex program")
            || trimmed.contains("tex engine");
        if is_magic_comment && (trimmed.contains("lualatex") || trimmed.contains("luatex")) {
            return TexEngine::LuaLatex;
        }
        if is_magic_comment && (trimmed.contains("xelatex") || trimmed.contains("xetex")) {
            return TexEngine::XeLatex;
        }
    }

    if lowercase.contains("lualatex")
        || lowercase.contains("\\directlua")
        || lowercase.contains("\\usepackage{luacode}")
    {
        TexEngine::LuaLatex
    } else {
        TexEngine::XeLatex
    }
}

pub(crate) fn preferred_engine(input: &Path) -> Result<TexEngine, String> {
    let source = fs::read_to_string(input)
        .map_err(|error| format!("TeX 소스를 읽지 못했습니다: {error}"))?;
    Ok(preferred_engine_for_source(&source))
}

fn write_dvisvgm_wrapper(input: &Path, work: &Path) -> Result<PathBuf, String> {
    let source_name = input
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or("TeX 파일명이 올바르지 않습니다.")?;
    if source_name.contains('"') {
        return Err("파일명에 큰따옴표가 포함된 TeX 문서는 직접 렌더링할 수 없습니다.".into());
    }
    let wrapper = work.join("texkey-direct-wrapper.tex");
    fs::write(
        &wrapper,
        format!(
            "\\PassOptionsToPackage{{backend=dvisvgm}}{{expl3}}\n\
             \\PassOptionsToPackage{{dvisvgm}}{{graphics}}\n\
             \\PassOptionsToPackage{{dvisvgm}}{{xcolor}}\n\
             \\def\\pgfsysdriver{{pgfsys-dvisvgm.def}}\n\
             \\makeatletter\n\
             \\AtBeginDocument{{%\n\
               \\ifdefined\\directlua\n\
                 \\pagewidth=\\paperwidth\n\
                 \\pageheight=\\paperheight\n\
               \\fi\n\
               \\def\\pgfsys@text@to@black@hook{{%\n\
                 \\ifx\\tikz@textcolor\\pgfutil@empty\\else\n\
                   \\color{{\\tikz@textcolor}}%\n\
                 \\fi\n\
               }}\n\
               \\patchcmd{{\\endbeamerboxesrounded}}\n\
                 {{\\unhbox\\bmb@colorbox}}\n\
                 {{\\unhbox\\bmb@colorbox\n\
                   \\let\\texkey@pgfusepath\\pgfusepath\n\
                   \\def\\pgfusepath#1{{%\n\
                     \\texkey@pgfusepath{{#1}}%\n\
                     \\pgfsys@invoke{{</g>}}%\n\
                     \\let\\pgfusepath\\texkey@pgfusepath\n\
                   }}%\n\
                 }}%\n\
                 {{\\typeout{{TEXKEY: patched Beamer rounded blocks for dvisvgm}}}}%\n\
                 {{\\PackageWarning{{texkey}}{{Could not patch Beamer rounded blocks}}}}%\n\
             }}\n\
             \\makeatother\n\
             \\input{{\"{source_name}\"}}\n"
        ),
    )
    .map_err(|error| error.to_string())?;
    Ok(wrapper)
}

fn prepare_dvisvgm_source(input: &Path, work: &Path) -> Result<PathBuf, String> {
    let source_directory = input.parent().unwrap_or_else(|| Path::new("."));
    for entry in fs::read_dir(source_directory).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let destination = work.join(entry.file_name());
        if !destination.exists() {
            symlink(entry.path(), destination).map_err(|error| {
                format!("TeX 보조 파일을 직접 렌더링 작업공간에 연결하지 못했습니다: {error}")
            })?;
        }
    }

    write_dvisvgm_wrapper(input, work)
}

pub(crate) struct Runtime<'a> {
    pub(crate) xelatex: &'a Path,
    pub(crate) dvilualatex: &'a Path,
    pub(crate) dvisvgm: &'a Path,
    /// Absent when Ghostscript is not installed. Only EPS and PostScript
    /// specials require it; documents without them render identically.
    pub(crate) ghostscript_library: Option<&'a Path>,
    /// Set for a bundled dvisvgm, which cannot derive the TeX tree location
    /// from its own path the way an in-tree copy does.
    pub(crate) texmf_root: Option<&'a Path>,
    pub(crate) home_directory: &'a Path,
}

pub fn manifest(
    input: &Path,
    output_directory: &Path,
    runtime: Runtime<'_>,
) -> Result<ManifestResult, String> {
    fs::create_dir_all(output_directory).map_err(|error| error.to_string())?;
    let wrapper = prepare_dvisvgm_source(input, output_directory)?;
    let engine = preferred_engine(input)?;
    let engine_path = match engine {
        TexEngine::XeLatex => runtime.xelatex,
        TexEngine::LuaLatex => runtime.dvilualatex,
    };

    for pass in 1..=2 {
        let mut typeset_command = Command::new(engine_path);
        configure_tex_process(&mut typeset_command, runtime.home_directory);
        typeset_command
            .current_dir(output_directory)
            .args([
                "-interaction=nonstopmode",
                "-halt-on-error",
                "-file-line-error",
            ])
            .arg(format!("-output-directory={}", output_directory.display()));
        if engine == TexEngine::XeLatex {
            typeset_command.arg("-no-pdf");
        }
        let typeset_output = typeset_command
            .arg(&wrapper)
            .output()
            .map_err(|error| format!("{}을 실행하지 못했습니다: {error}", engine.display_name()))?;
        if !typeset_output.status.success() {
            return Err(command_error(
                &format!("Beamer 문서 {pass}차 조판"),
                &typeset_output,
            ));
        }
    }

    let vector_input = output_directory.join(format!(
        "texkey-direct-wrapper.{}",
        engine.output_extension()
    ));
    if !vector_input.is_file() {
        return Err(format!(
            "{}이 {} 렌더링 결과를 만들지 않았습니다.",
            engine.display_name(),
            engine.output_extension().to_ascii_uppercase()
        ));
    }

    let output_pattern = output_directory.join("slide-%p.svg");
    let mut final_command = Command::new(runtime.dvisvgm);
    configure_tex_process(&mut final_command, runtime.home_directory);
    if let Some(root) = runtime.texmf_root {
        final_command
            .env("TEXMFROOT", root)
            .env("TEXMFCNF", root.join("texmf-dist/web2c"));
    }
    if let Some(library) = runtime.ghostscript_library {
        final_command.arg(format!("--libgs={}", library.to_string_lossy()));
    }
    let dvisvgm_output = final_command
        .current_dir(output_directory)
        .args([
            "--page=1-",
            "--bbox=papersize",
            "--no-fonts=0",
            "--embed-bitmaps",
        ])
        .arg(format!("--output={}", output_pattern.to_string_lossy()))
        .arg(&vector_input)
        .output()
        .map_err(|error| format!("벡터 엔진을 실행하지 못했습니다: {error}"))?;
    if !dvisvgm_output.status.success() {
        return Err(command_error("슬라이드 벡터 렌더링", &dvisvgm_output));
    }
    let dvisvgm_diagnostics = format!(
        "{}\n{}",
        String::from_utf8_lossy(&dvisvgm_output.stdout),
        String::from_utf8_lossy(&dvisvgm_output.stderr)
    );
    if dvisvgm_diagnostics.contains("PostScript specials ignored") {
        return Err(
            "Ghostscript가 일부 EPS/PostScript 요소를 SVG로 변환하지 못했습니다. Homebrew Ghostscript를 다시 설치한 뒤(`brew reinstall ghostscript`) TeXKey를 다시 실행해 주세요."
                .into(),
        );
    }

    let slides = svg_files(output_directory)?;
    for slide in &slides {
        embed_linked_bitmaps(slide)?;
    }
    let first_svg = fs::read_to_string(&slides[0]).map_err(|error| error.to_string())?;
    let width = attribute_number(&first_svg, "width")?.ceil() as u32;
    let height = attribute_number(&first_svg, "height")?.ceil() as u32;
    if width == 0 || height == 0 {
        return Err("렌더링된 슬라이드 크기가 올바르지 않습니다.".into());
    }

    let source_name = input.file_name().unwrap_or_default().to_string_lossy();
    let mut rows = Vec::with_capacity(slides.len() * 2);
    for (index, slide) in slides.iter().enumerate() {
        let page = index + 1;
        let svg = fs::read_to_string(slide).map_err(|error| error.to_string())?;
        let slide_width = attribute_number(&svg, "width")?.ceil() as u32;
        let slide_height = attribute_number(&svg, "height")?.ceil() as u32;
        if slide_width != width || slide_height != height {
            return Err(format!(
                "{page}번째 슬라이드의 크기가 첫 슬라이드와 다릅니다."
            ));
        }
        rows.push(format!(
            "SLIDE\t{page}\tSource: {source_name}, slide {page} · direct TeX vector"
        ));
        rows.push(format!("BACKGROUND\t{page}\t{}", slide.to_string_lossy()));
    }

    let manifest_path = output_directory.join("manifest.tsv");
    fs::write(&manifest_path, rows.join("\n") + "\n").map_err(|error| error.to_string())?;
    Ok(ManifestResult {
        manifest_path: manifest_path.to_string_lossy().into_owned(),
        width,
        height,
        pages: slides.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::{Runtime, TexEngine, attribute_number, manifest, preferred_engine_for_source};
    use crate::process_environment::ghostscript_library_path;
    use std::path::PathBuf;

    #[test]
    fn reads_svg_point_dimensions() {
        let svg = "<svg width='362.83473pt' height='272.126147pt'>";
        assert_eq!(attribute_number(svg, "width").unwrap().ceil() as u32, 363);
        assert_eq!(attribute_number(svg, "height").unwrap().ceil() as u32, 273);
    }

    #[test]
    fn selects_lualatex_from_source_comment() {
        let source = "% Build twice with LuaLaTeX.\n\\documentclass{beamer}";
        assert_eq!(preferred_engine_for_source(source), TexEngine::LuaLatex);
    }

    #[test]
    fn honors_tex_engine_magic_comment() {
        let source = "% !TeX program = xelatex\n% LuaLaTeX is also supported.";
        assert_eq!(preferred_engine_for_source(source), TexEngine::XeLatex);
    }

    #[test]
    fn uses_xelatex_by_default() {
        assert_eq!(
            preferred_engine_for_source("\\documentclass{beamer}"),
            TexEngine::XeLatex
        );
    }

    #[test]
    #[ignore = "requires TEXKEY_TEST_TEX and a system MacTeX installation"]
    fn renders_beamer_without_pdf() {
        let input = PathBuf::from(std::env::var("TEXKEY_TEST_TEX").unwrap());
        let output = std::env::temp_dir().join("texkey-tex-engine-test");
        if output.exists() {
            std::fs::remove_dir_all(&output).unwrap();
        }
        let texbin = PathBuf::from("/Library/TeX/texbin");
        let xelatex = texbin.join("xelatex");
        let dvilualatex = texbin.join("dvilualatex");
        let dvisvgm = texbin.join("dvisvgm");
        let ghostscript_library = ghostscript_library_path();
        let home_directory = PathBuf::from(std::env::var("HOME").unwrap());
        let result = manifest(
            &input,
            &output,
            Runtime {
                xelatex: &xelatex,
                dvilualatex: &dvilualatex,
                dvisvgm: &dvisvgm,
                ghostscript_library: ghostscript_library.as_deref(),
                texmf_root: None,
                home_directory: &home_directory,
            },
        )
        .unwrap();
        assert!(result.pages > 0);
        assert!(!output.join("texkey-direct-wrapper.pdf").exists());
        assert!(std::fs::read_dir(&output).unwrap().any(|entry| {
            entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with("slide-")
        }));
    }
}
