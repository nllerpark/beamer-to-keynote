use crate::{EmbeddedFontInfo, ManifestResult, PdfInfo};
use pdfium_render::prelude::*;
use std::{
    collections::{BTreeMap, BTreeSet, hash_map::DefaultHasher},
    fs,
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
};

const SCALE: f32 = 2.0;
const HORIZONTAL_EPSILON: f32 = 0.002;
const INFERRED_SPACE_EM: f32 = 0.10;

#[derive(Clone, Debug)]
struct CharacterInfo {
    value: char,
    font: String,
    size: f32,
    color: (u8, u8, u8),
    origin_x: f32,
    origin_y: f32,
    bbox: (f32, f32, f32, f32),
    angle: f32,
}

#[derive(Clone, Debug)]
struct TextRun {
    text: String,
    font: String,
    size: f32,
    color: (u8, u8, u8),
    bbox: (f32, f32, f32, f32),
    angle: f32,
}

fn pdfium(library_path: &Path) -> Result<Pdfium, String> {
    match Pdfium::bind_to_library(library_path) {
        Ok(bindings) => Ok(Pdfium::new(bindings)),
        Err(PdfiumError::PdfiumLibraryBindingsAlreadyInitialized) => Ok(Pdfium::default()),
        Err(error) => Err(format!("PDFium을 불러오지 못했습니다: {error}")),
    }
}

fn strip_subset_prefix(font: &str) -> String {
    let bytes = font.as_bytes();
    if bytes.len() > 7
        && bytes[6] == b'+'
        && bytes[..6].iter().all(|value| value.is_ascii_uppercase())
    {
        font[7..].to_owned()
    } else {
        font.to_owned()
    }
}

fn normalized_font_name(font: &str) -> String {
    font.chars()
        .filter(|character| character.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn is_font_installed(font: &str, installed_fonts: &BTreeSet<String>) -> bool {
    let requested = normalized_font_name(&strip_subset_prefix(font));
    installed_fonts
        .iter()
        .any(|installed| normalized_font_name(installed) == requested)
}

fn font_program_extension(data: &[u8]) -> Option<&'static str> {
    match data.get(..4) {
        Some(b"OTTO") => Some("otf"),
        Some(b"true") | Some(b"typ1") | Some([0, 1, 0, 0]) => Some("ttf"),
        _ => None,
    }
}

fn safe_font_filename(name: &str) -> String {
    let sanitized: String = name
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_') {
                character
            } else {
                '-'
            }
        })
        .take(80)
        .collect();
    if sanitized.is_empty() {
        "EmbeddedFont".into()
    } else {
        sanitized
    }
}

fn character_info(
    character: &PdfPageTextChar<'_>,
    page_height: f32,
    value: char,
) -> Option<CharacterInfo> {
    let bounds = character
        .loose_bounds()
        .or_else(|_| character.tight_bounds())
        .ok()?;
    let origin = character.origin().ok()?;
    let color = character.fill_color().unwrap_or(PdfColor::BLACK);
    // pdfium-render's scaled_font_size() multiplies by matrix.d(), which
    // becomes zero for exactly 90°/270° text. The transformed vertical axis
    // is the (c, d) vector, so its Euclidean length is the actual scale.
    let unscaled_size = character.unscaled_font_size().value.abs();
    let transformed_size = character
        .matrix()
        .map(|matrix| unscaled_size * matrix.c().hypot(matrix.d()))
        .unwrap_or_else(|_| character.scaled_font_size().value.abs());
    let fallback_size = (bounds.width().value.max(bounds.height().value) * 0.8).max(1.0);
    let size = if transformed_size.is_finite() && transformed_size > 0.01 {
        transformed_size
    } else if unscaled_size.is_finite() && unscaled_size > 0.01 {
        unscaled_size
    } else {
        fallback_size
    };
    Some(CharacterInfo {
        value,
        font: character.font_name(),
        size,
        color: (color.red(), color.green(), color.blue()),
        origin_x: origin.0.value,
        origin_y: page_height - origin.1.value,
        bbox: (
            bounds.left().value,
            page_height - bounds.top().value,
            bounds.right().value,
            page_height - bounds.bottom().value,
        ),
        angle: character.angle_degrees().unwrap_or(0.0),
    })
}

fn same_span(left: &CharacterInfo, right: &CharacterInfo) -> bool {
    let radians = left.angle.to_radians();
    let left_baseline = -radians.sin() * left.origin_x + radians.cos() * left.origin_y;
    let right_baseline = -radians.sin() * right.origin_x + radians.cos() * right.origin_y;
    left.font == right.font
        && (left.size - right.size).abs() <= 0.01
        && left.color == right.color
        && (left.angle - right.angle).abs() <= 0.01
        && (left_baseline - right_baseline).abs() <= left.size.max(right.size) * 0.35
}

fn finish_run(characters: &[CharacterInfo]) -> Option<TextRun> {
    if characters.is_empty() {
        return None;
    }
    let first = &characters[0];
    let threshold = 0.6_f32.max(first.size * INFERRED_SPACE_EM);
    let mut text = String::new();
    let mut visible: Vec<&CharacterInfo> = Vec::new();
    let mut previous: Option<&CharacterInfo> = None;

    for character in characters {
        if character.value.is_whitespace() {
            if !text.is_empty() && !text.ends_with(' ') {
                text.push(' ');
            }
            continue;
        }
        if let Some(previous) = previous
            && first.angle.abs() <= HORIZONTAL_EPSILON
            && character.origin_x - previous.bbox.2 > threshold
            && !text.is_empty()
            && !text.ends_with(' ')
        {
            text.push(' ');
        }
        text.push(character.value);
        visible.push(character);
        previous = Some(character);
    }

    let text = text.trim().to_owned();
    if text.is_empty() || visible.is_empty() {
        return None;
    }
    let bbox = visible.iter().fold(
        (
            f32::INFINITY,
            f32::INFINITY,
            f32::NEG_INFINITY,
            f32::NEG_INFINITY,
        ),
        |result, character| {
            (
                result.0.min(character.bbox.0),
                result.1.min(character.bbox.1),
                result.2.max(character.bbox.2),
                result.3.max(character.bbox.3),
            )
        },
    );
    Some(TextRun {
        text,
        font: first.font.clone(),
        size: first.size,
        color: first.color,
        bbox,
        angle: first.angle,
    })
}

fn extract_text_runs(page: &PdfPage<'_>) -> Result<Vec<TextRun>, String> {
    let page_height = page.height().value;
    let text = page.text().map_err(|error| error.to_string())?;
    let characters = text.chars();
    let mut runs = Vec::new();
    let mut span: Vec<CharacterInfo> = Vec::new();
    let mut index = 0;

    while index < characters.len() {
        let character = characters.get(index).map_err(|error| error.to_string())?;
        let unicode = character.unicode_value();
        let (value, consumed) =
            if (0xD800..=0xDBFF).contains(&unicode) && index + 1 < characters.len() {
                let low = characters
                    .get(index + 1)
                    .map_err(|error| error.to_string())?
                    .unicode_value();
                if (0xDC00..=0xDFFF).contains(&low) {
                    let codepoint = 0x10000 + ((unicode - 0xD800) << 10) + (low - 0xDC00);
                    (char::from_u32(codepoint), 2)
                } else {
                    (None, 1)
                }
            } else {
                (char::from_u32(unicode), 1)
            };
        index += consumed;
        let Some(value) = value else {
            continue;
        };
        if value == '\r' || value == '\n' || value == '\u{2028}' || value == '\u{2029}' {
            if let Some(run) = finish_run(&span) {
                runs.push(run);
            }
            span.clear();
            continue;
        }
        let Some(info) = character_info(&character, page_height, value) else {
            continue;
        };
        let starts_new_span = span
            .last()
            .is_some_and(|previous| !same_span(previous, &info));
        if starts_new_span {
            if let Some(run) = finish_run(&span) {
                runs.push(run);
            }
            span.clear();
        }
        span.push(info);
    }
    if let Some(run) = finish_run(&span) {
        runs.push(run);
    }
    Ok(runs)
}

fn remove_text_objects<'a, T>(objects: &mut T) -> Result<(), String>
where
    T: PdfPageObjectsCommon<'a>,
{
    let mut index = objects.len();
    while index > 0 {
        index -= 1;
        let object_type = objects
            .get(index)
            .map_err(|error| error.to_string())?
            .object_type();
        if object_type == PdfPageObjectType::Text {
            let removed = objects
                .remove_object_at_index(index)
                .map_err(|error| error.to_string())?;
            // PDFium 7881 can double-free an imported font resource when a detached
            // text object is destroyed immediately. The object is no longer attached
            // to the page, so keep its native allocation alive until process exit.
            std::mem::forget(removed);
        } else if object_type == PdfPageObjectType::XObjectForm {
            let mut object = objects.get(index).map_err(|error| error.to_string())?;
            if let Some(form) = object.as_x_object_form_object_mut() {
                remove_text_objects(form)?;
            }
        }
    }
    Ok(())
}

fn write_text_free_background(
    pdfium: &Pdfium,
    source: &mut PdfDocument<'_>,
    page_index: PdfPageIndex,
    output: &Path,
) -> Result<(), String> {
    {
        let mut page = source
            .pages_mut()
            .get(page_index)
            .map_err(|error| error.to_string())?;
        page.set_content_regeneration_strategy(PdfPageContentRegenerationStrategy::Manual);
        remove_text_objects(page.objects_mut())?;
        page.regenerate_content()
            .map_err(|error| error.to_string())?;
    }
    let mut background = pdfium.create_new_pdf().map_err(|error| error.to_string())?;
    background
        .pages_mut()
        .copy_page_from_document(source, page_index, 0)
        .map_err(|error| error.to_string())?;
    background
        .save_to_file(output)
        .map_err(|error| error.to_string())
}

fn write_complete_page_image(
    pdfium: &Pdfium,
    source: &PdfDocument<'_>,
    page_index: PdfPageIndex,
    output: &Path,
) -> Result<(), String> {
    let mut page_image = pdfium.create_new_pdf().map_err(|error| error.to_string())?;
    page_image
        .pages_mut()
        .copy_page_from_document(source, page_index, 0)
        .map_err(|error| error.to_string())?;
    page_image
        .save_to_file(output)
        .map_err(|error| error.to_string())
}

fn escaped_manifest_text(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character == '\t'
                || character == '\n'
                || character == '\r'
                || character == '\u{0085}'
                || character == '\u{2028}'
                || character == '\u{2029}'
                || character.is_control()
            {
                ' '
            } else {
                character
            }
        })
        .collect()
}

pub fn inspect(
    input: &Path,
    library_path: &Path,
    keynote_installed: bool,
    installed_fonts: &BTreeSet<String>,
) -> Result<PdfInfo, String> {
    let pdfium = pdfium(library_path)?;
    let document = pdfium
        .load_pdf_from_file(input, None)
        .map_err(|error| format!("PDF를 열 수 없습니다: {error}"))?;
    let first = document
        .pages()
        .first()
        .map_err(|error| error.to_string())?;
    let mut fonts = BTreeSet::new();
    let mut raw_fonts = BTreeMap::new();
    let mut text_items = 0;
    for page in document.pages().iter() {
        let runs = extract_text_runs(&page)?;
        text_items += runs.len();
        for run in runs {
            let display_name = strip_subset_prefix(&run.font);
            fonts.insert(display_name.clone());
            raw_fonts.entry(run.font).or_insert(display_name);
        }
    }
    let mut embedded_fonts = BTreeSet::new();
    for page in document.pages().iter() {
        let text = page.text().map_err(|error| error.to_string())?;
        let characters = text.chars();
        for index in 0..characters.len() {
            let character = characters.get(index).map_err(|error| error.to_string())?;
            let raw_name = character.font_name();
            if raw_fonts.contains_key(&raw_name)
                && !is_font_installed(&raw_name, installed_fonts)
                && !embedded_fonts.contains(&raw_name)
                && character
                    .text_object()
                    .ok()
                    .and_then(|object| object.font().is_embedded().ok())
                    == Some(true)
            {
                embedded_fonts.insert(raw_name);
            }
        }
    }
    let missing_fonts: BTreeSet<_> = raw_fonts
        .iter()
        .filter(|(raw, _)| !is_font_installed(raw, installed_fonts))
        .map(|(_, display)| display.clone())
        .collect();
    let extractable_fonts: BTreeMap<_, _> = embedded_fonts
        .into_iter()
        .filter_map(|raw| {
            raw_fonts.get(&raw).map(|name| {
                (
                    name.clone(),
                    EmbeddedFontInfo {
                        name: name.clone(),
                        subset: raw != *name,
                    },
                )
            })
        })
        .collect();
    Ok(PdfInfo {
        name: input
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned(),
        pages: document.pages().len() as usize,
        width: first.width().value.round() as u32,
        height: first.height().value.round() as u32,
        file_size: fs::metadata(input).map(|value| value.len()).unwrap_or(0),
        text_items,
        fonts: fonts.into_iter().collect(),
        missing_fonts: missing_fonts.into_iter().collect(),
        extractable_fonts: extractable_fonts.into_values().collect(),
        keynote_installed,
    })
}

pub(crate) struct FontExtractionReport {
    pub(crate) installed: usize,
    pub(crate) skipped: Vec<String>,
    pub(crate) preserved_pdf_font_names: BTreeSet<String>,
}

pub fn extract_embedded_fonts(
    input: &Path,
    library_path: &Path,
    font_directory: &Path,
    installed_fonts: &BTreeSet<String>,
) -> Result<FontExtractionReport, String> {
    const MAX_FONT_BYTES: usize = 32 * 1024 * 1024;
    const MAX_TOTAL_BYTES: usize = 128 * 1024 * 1024;

    let pdfium = pdfium(library_path)?;
    let document = pdfium
        .load_pdf_from_file(input, None)
        .map_err(|error| format!("PDF를 열 수 없습니다: {error}"))?;
    fs::create_dir_all(font_directory)
        .map_err(|error| format!("서체 설치 폴더를 만들지 못했습니다: {error}"))?;

    let mut handled = BTreeSet::new();
    let mut preserved_pdf_font_names = BTreeSet::new();
    let mut skipped = BTreeSet::new();
    let mut installed = 0;
    let mut total_bytes = 0usize;

    for page in document.pages().iter() {
        let text = page.text().map_err(|error| error.to_string())?;
        let characters = text.chars();
        for index in 0..characters.len() {
            let character = characters.get(index).map_err(|error| error.to_string())?;
            let raw_name = character.font_name();
            if raw_name.is_empty()
                || is_font_installed(&raw_name, installed_fonts)
                || !handled.insert(raw_name.clone())
            {
                continue;
            }
            let display_name = strip_subset_prefix(&raw_name);
            let Ok(object) = character.text_object() else {
                skipped.insert(display_name);
                continue;
            };
            let font = object.font();
            if font.is_embedded().ok() != Some(true) {
                skipped.insert(display_name);
                continue;
            }
            let data = font.data().map_err(|error| {
                format!("{display_name} 서체 데이터를 읽지 못했습니다: {error}")
            })?;
            let Some(extension) = font_program_extension(&data) else {
                skipped.insert(display_name);
                continue;
            };
            if data.is_empty()
                || data.len() > MAX_FONT_BYTES
                || total_bytes.saturating_add(data.len()) > MAX_TOTAL_BYTES
            {
                skipped.insert(display_name);
                continue;
            }

            let mut hasher = DefaultHasher::new();
            data.hash(&mut hasher);
            let filename = format!(
                "TeXKey-PDF-{}-{:016x}.{extension}",
                safe_font_filename(&raw_name),
                hasher.finish()
            );
            let destination = font_directory.join(filename);
            let needs_write = fs::metadata(&destination)
                .map(|metadata| metadata.len() != data.len() as u64)
                .unwrap_or(true);
            if needs_write {
                fs::write(&destination, &data).map_err(|error| {
                    format!("{display_name} 서체를 설치하지 못했습니다: {error}")
                })?;
                installed += 1;
            }
            total_bytes += data.len();
            preserved_pdf_font_names.insert(raw_name);
        }
    }

    Ok(FontExtractionReport {
        installed,
        skipped: skipped.into_iter().collect(),
        preserved_pdf_font_names,
    })
}

pub fn manifest(
    input: &Path,
    output_directory: &Path,
    library_path: &Path,
    preserved_pdf_font_names: &BTreeSet<String>,
) -> Result<ManifestResult, String> {
    fs::create_dir_all(output_directory).map_err(|error| error.to_string())?;
    let pdfium = pdfium(library_path)?;
    let document = pdfium
        .load_pdf_from_file(input, None)
        .map_err(|error| format!("PDF를 열 수 없습니다: {error}"))?;
    let mut background_document = pdfium
        .load_pdf_from_file(input, None)
        .map_err(|error| format!("배경용 PDF를 열 수 없습니다: {error}"))?;
    let first = document
        .pages()
        .first()
        .map_err(|error| error.to_string())?;
    let first_width = first.width().value;
    let first_height = first.height().value;
    let mut rows = Vec::new();

    for (page_index, page) in document.pages().iter().enumerate() {
        if (page.width().value - first_width).abs() > 0.01
            || (page.height().value - first_height).abs() > 0.01
        {
            return Err(format!(
                "{}페이지의 크기가 첫 페이지와 다릅니다.",
                page_index + 1
            ));
        }
        let page_number = page_index + 1;
        rows.push(format!(
            "SLIDE\t{page_number}\tSource: {}, page {page_number}",
            input.file_name().unwrap_or_default().to_string_lossy()
        ));
        let background = output_directory.join(format!("page-{page_number:04}.pdf"));
        write_text_free_background(
            &pdfium,
            &mut background_document,
            page_index as PdfPageIndex,
            &background,
        )?;
        rows.push(format!(
            "BACKGROUND\t{page_number}\t{}",
            background.to_string_lossy()
        ));

        for run in extract_text_runs(&page)? {
            let manifest_font = if preserved_pdf_font_names.contains(&run.font) {
                run.font.clone()
            } else {
                strip_subset_prefix(&run.font)
            };
            let size = run.size * SCALE;
            let (row_type, x, y, width, height) = if run.angle.abs() <= HORIZONTAL_EPSILON {
                (
                    "TEXT",
                    (run.bbox.0 * SCALE).round() as i32,
                    (run.bbox.1 * SCALE).round() as i32,
                    (((run.bbox.2 - run.bbox.0) * SCALE + size).ceil() as i32).max(2),
                    (((run.bbox.3 - run.bbox.1) * SCALE + size * 0.35).ceil() as i32).max(2),
                )
            } else {
                let bbox_width = run.bbox.2 - run.bbox.0;
                let bbox_height = run.bbox.3 - run.bbox.1;
                let radians = run.angle.abs().to_radians();
                let cosine = radians.cos().abs();
                let sine = radians.sin().abs();
                let denominator = cosine.powi(2) - sine.powi(2);
                let (natural_width, natural_height) = if denominator.abs() > 0.05 {
                    (
                        ((bbox_width * cosine - bbox_height * sine) / denominator).abs(),
                        ((bbox_height * cosine - bbox_width * sine) / denominator).abs(),
                    )
                } else {
                    (bbox_width.hypot(bbox_height), run.size * 1.25)
                };
                let natural_width = natural_width.max(run.size * 0.5);
                let natural_height = natural_height.max(run.size * 1.05);
                let center_x = (run.bbox.0 + run.bbox.2) / 2.0;
                let center_y = (run.bbox.1 + run.bbox.3) / 2.0;
                let width = (((natural_width + run.size * 0.25) * SCALE).ceil() as i32).max(2);
                let height = (((natural_height + run.size * 0.25) * SCALE).ceil() as i32).max(2);
                (
                    "ROTATED_TEXT",
                    (center_x * SCALE - width as f32 / 2.0).round() as i32,
                    (center_y * SCALE - height as f32 / 2.0).round() as i32,
                    width,
                    height,
                )
            };
            let red = u16::from(run.color.0) * 257;
            let green = u16::from(run.color.1) * 257;
            let blue = u16::from(run.color.2) * 257;
            rows.push(format!(
                "{row_type}\t{page_number}\t@{}\t{}\t{size:.3}\t{red}\t{green}\t{blue}\t{x}\t{y}\t{width}\t{height}\t{:02x}{:02x}{:02x}\t{:.3}",
                escaped_manifest_text(&run.text),
                manifest_font,
                run.color.0,
                run.color.1,
                run.color.2,
                run.angle
            ));
        }
    }

    let manifest_path = output_directory.join("manifest.tsv");
    fs::write(&manifest_path, rows.join("\n") + "\n").map_err(|error| error.to_string())?;
    Ok(ManifestResult {
        manifest_path: manifest_path.to_string_lossy().into_owned(),
        width: (first_width * SCALE).ceil() as u32,
        height: (first_height * SCALE).ceil() as u32,
        pages: document.pages().len() as usize,
    })
}

pub fn exact_image_manifest(
    input: &Path,
    output_directory: &Path,
    library_path: &Path,
) -> Result<ManifestResult, String> {
    fs::create_dir_all(output_directory).map_err(|error| error.to_string())?;
    let pdfium = pdfium(library_path)?;
    let document = pdfium
        .load_pdf_from_file(input, None)
        .map_err(|error| format!("PDF를 열 수 없습니다: {error}"))?;
    let first = document
        .pages()
        .first()
        .map_err(|error| error.to_string())?;
    let first_width = first.width().value;
    let first_height = first.height().value;
    let mut rows = Vec::new();

    for (page_index, page) in document.pages().iter().enumerate() {
        if (page.width().value - first_width).abs() > 0.01
            || (page.height().value - first_height).abs() > 0.01
        {
            return Err(format!(
                "{}페이지의 크기가 첫 페이지와 다릅니다.",
                page_index + 1
            ));
        }
        let page_number = page_index + 1;
        rows.push(format!(
            "SLIDE\t{page_number}\tSource: {}, page {page_number} · exact image",
            input.file_name().unwrap_or_default().to_string_lossy()
        ));
        let page_image = output_directory.join(format!("page-{page_number:04}.pdf"));
        write_complete_page_image(&pdfium, &document, page_index as PdfPageIndex, &page_image)?;
        rows.push(format!(
            "BACKGROUND\t{page_number}\t{}",
            page_image.to_string_lossy()
        ));
    }

    let manifest_path = output_directory.join("manifest.tsv");
    fs::write(&manifest_path, rows.join("\n") + "\n").map_err(|error| error.to_string())?;
    Ok(ManifestResult {
        manifest_path: manifest_path.to_string_lossy().into_owned(),
        width: (first_width * SCALE).ceil() as u32,
        height: (first_height * SCALE).ceil() as u32,
        pages: document.pages().len() as usize,
    })
}

pub fn development_library_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources/pdfium/lib/libpdfium.dylib")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static PDFIUM_TEST_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    #[ignore = "requires TEXKEY_TEST_PDF"]
    fn converts_real_pdf_without_background_text() {
        let _pdfium_guard = PDFIUM_TEST_LOCK.lock().unwrap();
        let input = PathBuf::from(std::env::var("TEXKEY_TEST_PDF").unwrap());
        let library = development_library_path();
        let output =
            std::env::temp_dir().join(format!("texkey-pdfium-test-{}", std::process::id()));
        let result = manifest(&input, &output, &library, &BTreeSet::new()).unwrap();
        assert!(result.pages > 0);

        let engine = pdfium(&library).unwrap();
        for page_number in 1..=result.pages {
            let background = engine
                .load_pdf_from_file(&output.join(format!("page-{page_number:04}.pdf")), None)
                .unwrap();
            let page = background.pages().first().unwrap();
            assert_eq!(
                page.text().unwrap().chars().len(),
                0,
                "page {page_number} background still contains text"
            );
        }

        let manifest = fs::read_to_string(output.join("manifest.tsv")).unwrap();
        for row in manifest.lines() {
            let columns: Vec<_> = row.split('\t').collect();
            let fields = columns.len();
            assert!(
                matches!(fields, 3 | 14),
                "malformed manifest row ({fields} fields): {row}"
            );
            if matches!(columns.first(), Some(&"TEXT" | &"ROTATED_TEXT")) {
                let size: f32 = columns[4].parse().unwrap();
                assert!(size > 0.0, "non-positive text size: {row}");
            }
        }
    }

    #[test]
    #[ignore = "requires TEXKEY_TEST_PDF"]
    fn exact_image_manifest_preserves_complete_pages() {
        let _pdfium_guard = PDFIUM_TEST_LOCK.lock().unwrap();
        let input = PathBuf::from(std::env::var("TEXKEY_TEST_PDF").unwrap());
        let library = development_library_path();
        let output =
            std::env::temp_dir().join(format!("texkey-pdfium-exact-test-{}", std::process::id()));
        let result = exact_image_manifest(&input, &output, &library).unwrap();
        assert!(result.pages > 0);

        let engine = pdfium(&library).unwrap();
        let source = engine.load_pdf_from_file(&input, None).unwrap();
        for page_number in 1..=result.pages {
            let image = engine
                .load_pdf_from_file(&output.join(format!("page-{page_number:04}.pdf")), None)
                .unwrap();
            let source_page = source
                .pages()
                .get((page_number - 1) as PdfPageIndex)
                .unwrap();
            let image_page = image.pages().first().unwrap();
            assert_eq!(image.pages().len(), 1);
            assert_eq!(image_page.objects().len(), source_page.objects().len());
            assert_eq!(
                image_page.text().unwrap().chars().len(),
                source_page.text().unwrap().chars().len()
            );
        }

        let manifest = fs::read_to_string(output.join("manifest.tsv")).unwrap();
        assert!(
            manifest
                .lines()
                .all(|row| row.starts_with("SLIDE\t") || row.starts_with("BACKGROUND\t"))
        );
        assert_eq!(manifest.lines().count(), result.pages * 2);
    }
}
