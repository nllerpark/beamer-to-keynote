const dictionaries = {
  en: {
    "app.title": "TeXKey",
    "app.descriptor": "Beamer & PDF → Keynote",
    "appearance.label": "Appearance",
    "appearance.system": "Follow system appearance",
    "appearance.light": "Light appearance",
    "appearance.dark": "Dark appearance",
    "language.label": "Language",
    "language.system": "System",
    "language.english": "English",
    "language.korean": "한국어",
    "source.heading": "Source document",
    "source.description": "Begin with a Beamer source or an existing PDF slide deck.",
    "source.dropKicker": "SOURCE INPUT",
    "source.dropTitle": "Drop a Beamer TeX or PDF here",
    "source.dropDescription": "TeX is typeset directly to vector slides without an intermediate PDF.",
    "source.chooseFile": "Choose file",
    "source.selected": "Selected document",
    "source.preparing": "Preparing source…",
    "source.replace": "Choose another file",
    "source.clear": "Clear selected document",
    "source.invalid": "Choose a PDF or Beamer .tex file.",
    "inspection.heading": "Readiness inspection",
    "inspection.analyzing": "Analyzing",
    "inspection.ready": "Ready",
    "inspection.directReady": "Direct rendering ready",
    "inspection.textObjects": "Editable text objects",
    "inspection.slidePages": "Slide pages",
    "inspection.fonts": "Document fonts",
    "inspection.fontWarning": "Some fonts may be substituted.",
    "inspection.texFontWarning": "Required system fonts may be unavailable.",
    "inspection.texTextValue": "Outlines",
    "inspection.texPagesValue": "Automatic",
    "inspection.texFontsValue": "Included",
    "inspection.texFontNote": "Math and type are embedded as SVG paths, preserving the same appearance on other Macs.",
    "inspection.texSystemFontNote": "Explicit system fonts: {fonts}",
    "inspection.pdfFontNote": "Visuals remain in the PDF layer · Editable fonts: {fonts}",
    "inspection.noFonts": "No separate fonts were detected in this document.",
    "brief.heading": "Conversion brief",
    "brief.description": "A precise, self-contained Keynote document.",
    "brief.method": "Method",
    "brief.destination": "Destination",
    "brief.sameFolder": "Same folder as source",
    "brief.openAfter": "Open in Keynote after completion",
    "brief.openAfterHelp": "Show the finished deck immediately.",
    "method.auto": "Selected automatically",
    "method.autoHelp": "The available method follows the selected source.",
    "method.texDirect": "Direct vector · TeX native",
    "method.texDirectHelp": "Typesets DVI/XDV directly to vector slides without producing a PDF.",
    "method.editable": "Editable text",
    "method.exactImage": "Exact appearance · image",
    "method.editableHelp": "Restores text as editable Keynote objects over a vector PDF layer.",
    "method.exactImageHelp": "Places each complete PDF page as one sharp image layer.",
    "progress.preparing": "Preparing conversion…",
    "progress.texTitle": "Typesetting Beamer directly…",
    "progress.texDetail": "Generating slide vectors from DVI/XDV without creating a PDF.",
    "progress.exactTitle": "Preserving the original pages…",
    "progress.exactDetail": "Preparing each PDF page as one complete, high-fidelity image layer.",
    "progress.editableTitle": "Reconstructing the PDF structure…",
    "progress.editableDetail": "Preparing page vectors and editable text objects.",
    "progress.keynoteTitle": "Building the Keynote document…",
    "progress.keynoteDetail": "macOS may ask for permission to automate Keynote.",
    "progress.complete": "Conversion complete",
    "action.convert": "Convert to Keynote",
    "action.note": "Nothing is uploaded. Conversion stays on this Mac.",
    "status.checking": "Checking",
    "status.ready": "Ready",
    "status.required": "Required",
    "status.active": "Active",
    "status.external": "MacTeX + Ghostscript ready",
    "status.texMissing": "MacTeX / Ghostscript required",
    "status.unavailable": "Unavailable",
    "common.details": "Details",
    "common.close": "Close",
    "common.cancel": "Cancel",
    "fontDialog.heading": "Missing fonts are embedded in this PDF",
    "fontDialog.description": "TeXKey can extract them for editable text, but font programs require special care.",
    "fontDialog.available": "Embedded fonts available to extract",
    "fontDialog.none": "No missing font in this PDF has an extractable embedded program.",
    "fontDialog.subsetSuffix": " (subset)",
    "fontDialog.licenseTitle": "License responsibility",
    "fontDialog.licenseBody": "The PDF may not grant permission to install or reuse its fonts. Confirm that you have the necessary rights.",
    "fontDialog.subsetTitle": "Subset limitations",
    "fontDialog.subsetBody": "Embedded subsets may contain only the glyphs used in this PDF, so newly typed characters can fall back.",
    "fontDialog.securityTitle": "Untrusted font data",
    "fontDialog.securityBody": "Font files can carry security risk. Extracted fonts are installed for your user account and remain after conversion.",
    "fontDialog.formatNote": "Only embedded OpenType or TrueType programs are installed. Non-embedded, Type 3, and unsupported fonts are skipped.",
    "fontDialog.skip": "Continue without extraction",
    "fontDialog.extract": "Extract fonts and continue",
    "texFontDialog.heading": "TeX system fonts require attention",
    "texFontDialog.description": "The source explicitly names fonts that TeXKey could not match to fonts installed on this Mac.",
    "texFontDialog.missing": "Fonts not found in macOS",
    "texFontDialog.guidanceTitle": "Install before typesetting",
    "texFontDialog.guidanceBody": "Install these families in Font Book, then select the source again. If the project supplies the font files itself, confirm that their paths are valid.",
    "texFontDialog.scopeTitle": "Static preflight",
    "texFontDialog.scopeBody": "TeXKey checks explicit fontspec and CJK font declarations. Fonts selected dynamically by macros can be diagnosed only by the typesetting log.",
    "texFontDialog.packageNote": "Fonts supplied by a TeX package may still resolve during typesetting. Continuing can nevertheless fail or produce a substituted typeface.",
    "texFontDialog.continue": "Continue anyway",
    "dialog.chooseSource": "Choose a PDF or Beamer TeX file to convert to Keynote",
    "dialog.chooseOutput": "Choose a folder for the Keynote document",
    "file.texPreparing": "Preparing Beamer source…",
    "file.pdfPreparing": "Analyzing PDF…",
    "file.texMeta": "Beamer TeX · direct DVI/XDV vector rendering without PDF",
    "file.pdfMeta": "{width} × {height} pt · {pages} pages · {size}",
    "count.items": "{count} items",
    "count.pages": "{count} pages",
    "count.fonts": "{count} fonts",
    "toast.tauriOnly": "Run this interface inside the TeXKey app.",
    "toast.fontHelp": "Install the missing fonts in Font Book for a closer editable-text match.",
    "toast.fontExtracted": "{count} embedded font files were installed for this user.",
    "toast.fontSkipped": "Unsupported or unavailable fonts were skipped: {fonts}",
    "toast.exactMode": "Each complete PDF page will be preserved as a single image layer.",
    "toast.editableMode": "Text will be reconstructed as editable Keynote objects.",
    "toast.converted": "Converted {pages} pages to a {kind} Keynote document.",
    "result.texDirect": "direct-vector",
    "result.exactImage": "pixel-faithful",
    "result.editable": "editable",
    "error.generic": "The operation could not be completed.",
    "error.noKeynote": "Keynote is not installed on this Mac.",
    "error.engine": "A required native conversion engine is unavailable.",
    "error.invalidSource": "Choose a valid PDF or Beamer .tex source file.",
    "error.keynoteAutomation": "TeXKey could not create the Keynote document: {detail}",
  },
  ko: {
    "app.title": "TeXKey",
    "app.descriptor": "Beamer·PDF → Keynote",
    "appearance.label": "화면 모드",
    "appearance.system": "시스템 화면 모드 따르기",
    "appearance.light": "라이트 모드",
    "appearance.dark": "다크 모드",
    "language.label": "언어",
    "language.system": "시스템",
    "language.english": "English",
    "language.korean": "한국어",
    "source.heading": "원본 문서",
    "source.description": "Beamer 소스나 기존 PDF 슬라이드에서 시작하세요.",
    "source.dropKicker": "SOURCE INPUT",
    "source.dropTitle": "Beamer TeX 또는 PDF를 놓으세요",
    "source.dropDescription": "TeX는 중간 PDF 없이 바로 벡터 슬라이드로 조판합니다.",
    "source.chooseFile": "파일 선택",
    "source.selected": "선택한 문서",
    "source.preparing": "원본 준비 중…",
    "source.replace": "다른 파일 선택",
    "source.clear": "선택한 문서 지우기",
    "source.invalid": "PDF 또는 Beamer .tex 파일을 선택해 주세요.",
    "inspection.heading": "변환 준비 점검",
    "inspection.analyzing": "분석 중",
    "inspection.ready": "준비됨",
    "inspection.directReady": "직접 렌더링 준비됨",
    "inspection.textObjects": "편집 가능한 텍스트",
    "inspection.slidePages": "슬라이드 페이지",
    "inspection.fonts": "문서 서체",
    "inspection.fontWarning": "일부 서체가 대체될 수 있습니다.",
    "inspection.texFontWarning": "필요한 시스템 서체가 없을 수 있습니다.",
    "inspection.texTextValue": "윤곽선",
    "inspection.texPagesValue": "자동",
    "inspection.texFontsValue": "포함",
    "inspection.texFontNote": "수식과 서체를 SVG 경로로 포함하여 다른 Mac에서도 같은 모양을 유지합니다.",
    "inspection.texSystemFontNote": "명시한 시스템 서체: {fonts}",
    "inspection.pdfFontNote": "원본 모양은 PDF 레이어에 포함 · 편집 서체: {fonts}",
    "inspection.noFonts": "문서에서 별도 서체를 찾지 못했습니다.",
    "brief.heading": "변환 명세",
    "brief.description": "정확하고 독립적인 Keynote 문서를 만듭니다.",
    "brief.method": "변환 방식",
    "brief.destination": "저장 위치",
    "brief.sameFolder": "원본과 같은 폴더",
    "brief.openAfter": "완료 후 Keynote에서 열기",
    "brief.openAfterHelp": "완성된 슬라이드를 바로 표시합니다.",
    "method.auto": "원본에 따라 자동 선택",
    "method.autoHelp": "선택한 원본에 맞는 변환 방식이 표시됩니다.",
    "method.texDirect": "직접 벡터 · TeX 네이티브",
    "method.texDirectHelp": "PDF를 만들지 않고 DVI/XDV에서 벡터 슬라이드를 직접 생성합니다.",
    "method.editable": "텍스트 편집 가능",
    "method.exactImage": "원본 그대로 · 이미지",
    "method.editableHelp": "벡터 PDF 레이어 위에 텍스트를 편집 가능한 Keynote 객체로 복원합니다.",
    "method.exactImageHelp": "각 PDF 페이지 전체를 하나의 선명한 이미지 레이어로 넣습니다.",
    "progress.preparing": "변환 준비 중…",
    "progress.texTitle": "Beamer를 직접 조판하는 중…",
    "progress.texDetail": "PDF를 만들지 않고 DVI/XDV에서 슬라이드 벡터를 생성하고 있습니다.",
    "progress.exactTitle": "원본 페이지를 보존하는 중…",
    "progress.exactDetail": "각 PDF 페이지를 하나의 완전한 고품질 이미지 레이어로 준비합니다.",
    "progress.editableTitle": "PDF 구조를 복원하는 중…",
    "progress.editableDetail": "페이지 벡터와 편집 가능한 텍스트 객체를 준비합니다.",
    "progress.keynoteTitle": "Keynote 문서를 만드는 중…",
    "progress.keynoteDetail": "macOS가 Keynote 자동화 권한을 요청할 수 있습니다.",
    "progress.complete": "변환 완료",
    "action.convert": "Keynote로 변환",
    "action.note": "파일은 업로드되지 않으며 이 Mac에서만 변환됩니다.",
    "status.checking": "확인 중",
    "status.ready": "준비됨",
    "status.required": "설치 필요",
    "status.active": "활성",
    "status.external": "MacTeX + Ghostscript 준비됨",
    "status.texMissing": "MacTeX / Ghostscript 필요",
    "status.unavailable": "사용 불가",
    "common.details": "자세히",
    "common.close": "닫기",
    "common.cancel": "취소",
    "fontDialog.heading": "이 PDF에 누락 서체가 포함되어 있습니다",
    "fontDialog.description": "편집 텍스트를 위해 추출할 수 있지만, 서체 프로그램은 주의해서 다뤄야 합니다.",
    "fontDialog.available": "추출 가능한 PDF 내장 서체",
    "fontDialog.none": "누락된 서체 중 PDF에서 안전하게 꺼낼 수 있는 내장 프로그램이 없습니다.",
    "fontDialog.subsetSuffix": " (서브셋)",
    "fontDialog.licenseTitle": "라이선스 책임",
    "fontDialog.licenseBody": "PDF에 포함됐다는 사실만으로 설치·재사용 권한이 생기지는 않습니다. 필요한 권한이 있는지 확인하세요.",
    "fontDialog.subsetTitle": "서브셋 한계",
    "fontDialog.subsetBody": "내장 서체가 PDF에서 쓴 글자만 가진 서브셋이면 새로 입력한 글자는 다른 서체로 대체될 수 있습니다.",
    "fontDialog.securityTitle": "신뢰할 수 없는 서체 데이터",
    "fontDialog.securityBody": "서체 파일에도 보안 위험이 있을 수 있습니다. 추출한 서체는 사용자 계정에 설치되며 변환 후에도 남습니다.",
    "fontDialog.formatNote": "PDF에 실제로 내장된 OpenType·TrueType만 설치합니다. 비내장, Type 3, 미지원 형식은 건너뜁니다.",
    "fontDialog.skip": "추출하지 않고 계속",
    "fontDialog.extract": "서체 추출 후 계속",
    "texFontDialog.heading": "TeX 시스템 서체를 확인하세요",
    "texFontDialog.description": "TeX 원본에 명시된 서체 중 이 Mac에 설치된 서체와 일치하지 않는 항목이 있습니다.",
    "texFontDialog.missing": "macOS에서 찾지 못한 서체",
    "texFontDialog.guidanceTitle": "조판 전에 설치",
    "texFontDialog.guidanceBody": "서체 관리자에서 이 서체를 설치한 뒤 원본을 다시 선택하세요. 프로젝트가 서체 파일을 직접 제공한다면 파일 경로가 올바른지 확인하세요.",
    "texFontDialog.scopeTitle": "정적 사전 점검",
    "texFontDialog.scopeBody": "fontspec 및 CJK 명령에 직접 적힌 서체를 확인합니다. 매크로로 동적으로 선택한 서체는 조판 로그에서만 진단할 수 있습니다.",
    "texFontDialog.packageNote": "TeX 패키지가 제공하는 서체라면 조판 중에 확인될 수도 있습니다. 그래도 계속하면 변환이 실패하거나 대체 서체가 사용될 수 있습니다.",
    "texFontDialog.continue": "그래도 계속",
    "dialog.chooseSource": "Keynote로 변환할 PDF 또는 Beamer TeX를 선택하세요",
    "dialog.chooseOutput": "Keynote 파일을 저장할 폴더를 선택하세요",
    "file.texPreparing": "Beamer 소스 준비 중…",
    "file.pdfPreparing": "PDF 분석 중…",
    "file.texMeta": "Beamer TeX · PDF 없는 직접 DVI/XDV 벡터 렌더링",
    "file.pdfMeta": "{width} × {height} pt · {pages}페이지 · {size}",
    "count.items": "{count}개",
    "count.pages": "{count}페이지",
    "count.fonts": "{count}개",
    "toast.tauriOnly": "TeXKey 앱에서 실행해 주세요.",
    "toast.fontHelp": "누락된 서체를 macOS 서체 관리자에 설치하면 편집 텍스트가 원본과 더 정확히 일치합니다.",
    "toast.fontExtracted": "내장 서체 파일 {count}개를 이 사용자 계정에 설치했습니다.",
    "toast.fontSkipped": "추출할 수 없거나 지원하지 않는 서체는 건너뛰었습니다: {fonts}",
    "toast.exactMode": "각 PDF 페이지 전체를 하나의 이미지 레이어로 넣어 원본 모양을 보존합니다.",
    "toast.editableMode": "텍스트를 편집 가능한 Keynote 객체로 복원합니다.",
    "toast.converted": "{pages}페이지를 {kind} Keynote 문서로 변환했습니다.",
    "result.texDirect": "직접 벡터",
    "result.exactImage": "원본 그대로",
    "result.editable": "편집 가능한",
    "error.generic": "작업을 완료하지 못했습니다.",
    "error.noKeynote": "이 Mac에 Keynote가 설치되어 있지 않습니다.",
    "error.engine": "필요한 네이티브 변환 엔진을 사용할 수 없습니다.",
    "error.invalidSource": "올바른 PDF 또는 Beamer .tex 원본을 선택해 주세요.",
    "error.keynoteAutomation": "Keynote 문서를 만들지 못했습니다: {detail}",
  },
};

function detectLocale() {
  const preferred = navigator.languages?.length ? navigator.languages : [navigator.language];
  const first = preferred.find(Boolean)?.toLowerCase() || "en";
  return first === "ko" || first.startsWith("ko-") ? "ko" : "en";
}

const languageStorageKey = "texkey.language";
const languagePreferences = new Set(["system", "en", "ko"]);

export function getLanguagePreference() {
  try {
    const preference = localStorage.getItem(languageStorageKey);
    return languagePreferences.has(preference) ? preference : "system";
  } catch {
    return "system";
  }
}

function resolveLocale(preference = getLanguagePreference()) {
  return preference === "system" ? detectLocale() : preference;
}

export let locale = resolveLocale();

export function setLanguagePreference(value) {
  const preference = languagePreferences.has(value) ? value : "system";
  try {
    localStorage.setItem(languageStorageKey, preference);
  } catch {
    // The selected locale still applies for this session when storage is unavailable.
  }
  locale = resolveLocale(preference);
  return locale;
}

export function t(key, values = {}) {
  const template = dictionaries[locale][key] ?? dictionaries.en[key] ?? key;
  return template.replace(/\{(\w+)\}/g, (_, name) => String(values[name] ?? `{${name}}`));
}

export function applyTranslations(root = document) {
  document.documentElement.lang = locale;
  document.title = t("app.title");

  root.querySelectorAll("[data-i18n]").forEach((element) => {
    element.textContent = t(element.dataset.i18n);
  });
  root.querySelectorAll("[data-i18n-aria]").forEach((element) => {
    element.setAttribute("aria-label", t(element.dataset.i18nAria));
  });
  root.querySelectorAll("[data-i18n-title]").forEach((element) => {
    element.setAttribute("title", t(element.dataset.i18nTitle));
  });
}

export function formatInteger(value) {
  return new Intl.NumberFormat(locale).format(value);
}
