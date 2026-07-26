use crate::ManifestResult;
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
            "\\PassOptionsToPackage{{dvisvgm}}{{graphics}}\n\
             \\PassOptionsToPackage{{dvisvgm}}{{xcolor}}\n\
             \\def\\pgfsysdriver{{pgfsys-dvisvgm.def}}\n\
             \\makeatletter\n\
             \\AtBeginDocument{{%\n\
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

fn configure_dvisvgm(command: &mut Command, texmf: &Path) {
    if !texmf.is_dir() {
        return;
    }
    command
        .env("TFMFONTS", format!("{}//", texmf.display()))
        .env("OPENTYPEFONTS", format!("{}//", texmf.display()))
        .env("T1FONTS", format!("{}//", texmf.display()))
        .env("TEXFONTMAPS", format!("{}//", texmf.display()))
        .env("ENCFONTS", format!("{}//", texmf.display()));
}

pub fn manifest(
    input: &Path,
    output_directory: &Path,
    tectonic: &Path,
    dvisvgm: &Path,
    bundle: &Path,
    dvisvgm_texmf: &Path,
    cache_directory: &Path,
) -> Result<ManifestResult, String> {
    fs::create_dir_all(output_directory).map_err(|error| error.to_string())?;
    fs::create_dir_all(cache_directory).map_err(|error| error.to_string())?;
    let wrapper = prepare_dvisvgm_source(input, output_directory)?;

    let run_tectonic = || {
        let mut command = Command::new(tectonic);
        command
            .current_dir(output_directory)
            .env("TECTONIC_CACHE_DIR", cache_directory)
            .args(["--color", "never"]);
        if bundle.is_file() {
            command.arg("--bundle").arg(bundle).arg("--only-cached");
        }
        command
            .args(["--outfmt", "xdv", "--keep-logs", "--outdir"])
            .arg(output_directory)
            .arg(&wrapper)
            .output()
            .map_err(|error| format!("TeX 엔진을 실행하지 못했습니다: {error}"))
    };
    let tectonic_output = run_tectonic()?;
    if !tectonic_output.status.success() {
        return Err(command_error("Beamer 문서 조판", &tectonic_output));
    }

    let xdv = output_directory.join("texkey-direct-wrapper.xdv");
    if !xdv.is_file() {
        return Err("TeX 엔진이 XDV 렌더링 결과를 만들지 않았습니다.".into());
    }

    let output_pattern = output_directory.join("slide-%p.svg");
    let mut final_command = Command::new(dvisvgm);
    configure_dvisvgm(&mut final_command, dvisvgm_texmf);
    let dvisvgm_output = final_command
        .current_dir(output_directory)
        .args(["--page=1-", "--bbox=papersize", "--no-fonts=0"])
        .arg(format!("--output={}", output_pattern.to_string_lossy()))
        .arg(&xdv)
        .output()
        .map_err(|error| format!("벡터 엔진을 실행하지 못했습니다: {error}"))?;
    if !dvisvgm_output.status.success() {
        return Err(command_error("슬라이드 벡터 렌더링", &dvisvgm_output));
    }

    let slides = svg_files(output_directory)?;
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
    use super::{attribute_number, manifest};
    use std::path::PathBuf;

    #[test]
    fn reads_svg_point_dimensions() {
        let svg = "<svg width='362.83473pt' height='272.126147pt'>";
        assert_eq!(attribute_number(svg, "width").unwrap().ceil() as u32, 363);
        assert_eq!(attribute_number(svg, "height").unwrap().ceil() as u32, 273);
    }

    #[test]
    #[ignore = "requires TEXKEY_TEST_TEX and bundled native engines"]
    fn renders_beamer_without_pdf() {
        let input = PathBuf::from(std::env::var("TEXKEY_TEST_TEX").unwrap());
        let project = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let output = std::env::temp_dir().join("texkey-tex-engine-test");
        if output.exists() {
            std::fs::remove_dir_all(&output).unwrap();
        }
        let result = manifest(
            &input,
            &output,
            &project.join("binaries/tectonic-aarch64-apple-darwin"),
            &project.join("binaries/dvisvgm-aarch64-apple-darwin"),
            &project.join("resources/tex-engines/texkey-curated.ttb"),
            &project.join("resources/dvisvgm-texmf"),
            &std::env::temp_dir().join("texkey-tex-engine-cache"),
        )
        .unwrap();
        assert!(result.pages > 0);
        assert!(!std::fs::read_dir(&output).unwrap().any(|entry| {
            entry
                .unwrap()
                .path()
                .extension()
                .is_some_and(|value| value.eq_ignore_ascii_case("pdf"))
        }));
        assert!(std::fs::read_dir(&output).unwrap().any(|entry| {
            entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with("slide-")
        }));
    }
}
