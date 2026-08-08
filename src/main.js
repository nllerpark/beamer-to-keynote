import {
  applyTranslations,
  formatInteger,
  getLanguagePreference,
  locale,
  setLanguagePreference,
  t,
} from "./i18n.js";

const invoke = window.__TAURI__?.core?.invoke;
const appWindow = window.__TAURI__?.window?.getCurrentWindow?.();
const state = {
  path: null,
  info: null,
  outputDir: null,
  busy: false,
  kind: null,
  env: null,
  fontDecision: null,
  texFontDecision: null,
  pendingConversion: false,
};

const $ = (id) => document.getElementById(id);
const elements = {
  empty: $("empty-state"),
  file: $("file-state"),
  drop: $("drop-zone"),
  sourcePane: document.querySelector(".source-pane"),
  fileName: $("file-name"),
  fileMeta: $("file-meta"),
  textCount: $("text-count"),
  pageCount: $("page-count"),
  fontCount: $("font-count"),
  fontList: $("font-list"),
  badge: $("readiness-badge"),
  warning: $("warning-row"),
  warningTitle: $("font-warning-title"),
  missingFonts: $("missing-fonts"),
  convert: $("convert-button"),
  clear: $("clear-button"),
  outputLabel: $("output-label"),
  progress: $("progress-panel"),
  progressBar: $("progress-bar"),
  progressTitle: $("progress-title"),
  progressDetail: $("progress-detail"),
  keynoteStatus: $("keynote-status"),
  pdfiumStatus: $("pdfium-status"),
  texStatus: $("tex-status"),
  toast: $("toast"),
  sourceKind: $("source-kind"),
  mode: $("conversion-mode"),
  modeHelp: $("mode-help"),
  fontDialog: $("font-risk-dialog"),
  extractableFontList: $("extractable-font-list"),
  extractFontButton: $("extract-font-button"),
  texFontDialog: $("tex-font-dialog"),
  texMissingFontList: $("tex-missing-font-list"),
  language: $("language-select"),
};

const appearanceValues = new Set(["system", "light", "dark"]);
const appearanceStorageKey = "texkey.appearance";

function savedAppearance() {
  try {
    const value = localStorage.getItem(appearanceStorageKey);
    return appearanceValues.has(value) ? value : "system";
  } catch {
    return "system";
  }
}

function setAppearance(value, persist = true) {
  const appearance = appearanceValues.has(value) ? value : "system";
  document.documentElement.dataset.theme = appearance;
  document.documentElement.style.colorScheme = appearance === "system" ? "light dark" : appearance;

  document.querySelectorAll("[data-theme-value]").forEach((button) => {
    const selected = button.dataset.themeValue === appearance;
    button.classList.toggle("active", selected);
    button.setAttribute("aria-checked", String(selected));
  });

  if (persist) {
    try {
      localStorage.setItem(appearanceStorageKey, appearance);
    } catch {
      // Appearance still applies for this session when storage is unavailable.
    }
  }
}

function formatBytes(bytes) {
  if (bytes < 1024 * 1024) {
    return `${Math.max(1, Math.round(bytes / 1024)).toLocaleString(locale)} KB`;
  }
  return `${new Intl.NumberFormat(locale, { maximumFractionDigits: 1 }).format(bytes / 1024 / 1024)} MB`;
}

function showToast(message, duration = 3800) {
  elements.toast.textContent = message;
  elements.toast.hidden = false;
  clearTimeout(showToast.timer);
  showToast.timer = setTimeout(() => {
    elements.toast.hidden = true;
  }, duration);
}

function setBadge(key, scanning = false) {
  elements.badge.className = scanning ? "badge scanning" : "badge";
  elements.badge.querySelector("span").textContent = t(key);
}

function option(value, label) {
  const item = document.createElement("option");
  item.value = value;
  item.textContent = label;
  return item;
}

function configureMode(kind) {
  elements.mode.replaceChildren();
  if (kind === "tex") {
    elements.mode.append(option("texDirect", t("method.texDirect")));
    elements.mode.disabled = true;
    elements.modeHelp.textContent = t("method.texDirectHelp");
    return;
  }
  if (kind === "pdf") {
    elements.mode.append(
      option("editable", t("method.editable")),
      option("exactImage", t("method.exactImage")),
    );
    elements.mode.disabled = false;
    elements.modeHelp.textContent = t("method.editableHelp");
    return;
  }
  elements.mode.append(option("auto", t("method.auto")));
  elements.mode.disabled = true;
  elements.modeHelp.textContent = t("method.autoHelp");
}

function renderSourceInfo() {
  if (!state.kind || !state.info) return;

  if (state.kind === "tex") {
    elements.fileName.textContent = state.info.name || state.path.split(/[\\/]/).pop();
    elements.fileMeta.textContent = t("file.texMeta");
    elements.textCount.textContent = t("inspection.texTextValue");
    elements.pageCount.textContent = t("inspection.texPagesValue");
    elements.fontCount.textContent = state.info.fonts?.length
      ? t("count.fonts", { count: formatInteger(state.info.fonts.length) })
      : t("inspection.texFontsValue");
    elements.fontList.textContent = state.info.fonts?.length
      ? t("inspection.texSystemFontNote", { fonts: state.info.fonts.join(" · ") })
      : t("inspection.texFontNote");
    setBadge("inspection.directReady");
    elements.warning.hidden = !state.info.missingFonts?.length;
    elements.warningTitle.textContent = t("inspection.texFontWarning");
    elements.missingFonts.textContent = state.info.missingFonts?.join(", ") || "";
    return;
  }

  elements.fileName.textContent = state.info.name;
  elements.fileMeta.textContent = t("file.pdfMeta", {
    width: state.info.width,
    height: state.info.height,
    pages: formatInteger(state.info.pages),
    size: formatBytes(state.info.fileSize),
  });
  elements.textCount.textContent = t("count.items", {
    count: formatInteger(state.info.textItems),
  });
  elements.pageCount.textContent = t("count.pages", {
    count: formatInteger(state.info.pages),
  });
  elements.fontCount.textContent = t("count.fonts", {
    count: formatInteger(state.info.fonts.length),
  });
  elements.fontList.textContent = state.info.fonts.length
    ? t("inspection.pdfFontNote", { fonts: state.info.fonts.join(" · ") })
    : t("inspection.noFonts");
  setBadge("inspection.ready");
  elements.warning.hidden = !state.info.missingFonts.length;
  elements.warningTitle.textContent = t("inspection.fontWarning");
  elements.missingFonts.textContent = state.info.missingFonts.join(", ");
}

function canConvert() {
  if (!state.path || state.busy || !state.info?.keynoteInstalled) return false;
  return state.kind !== "tex" || Boolean(state.env?.texEngineReady);
}

function updateConvertAvailability() {
  elements.convert.disabled = !canConvert();
}

function localizeBackendError(error) {
  const raw = String(error).replace(/^Error:\s*/, "").trim();
  if (locale === "ko") return raw;
  if (raw.includes("Keynote 앱이 설치되어 있지 않습니다")) return t("error.noKeynote");
  if (raw.includes("올바른 입력 파일") || raw.includes("올바른 PDF 파일")) {
    return t("error.invalidSource");
  }
  if (
    raw.includes("PDFium 네이티브 엔진")
    || raw.includes("직접 렌더링 엔진이 앱에 포함되지 않았습니다")
    || raw.includes("직접 렌더링 엔진을 찾지 못했습니다")
  ) {
    return t("error.engine");
  }
  if (raw.startsWith("Keynote 문서를 만들지 못했습니다:")) {
    return t("error.keynoteAutomation", {
      detail: raw.slice(raw.indexOf(":") + 1).trim(),
    });
  }
  return `${t("error.generic")} ${raw}`;
}

async function chooseSource() {
  if (!invoke) {
    showToast(t("toast.tauriOnly"));
    return;
  }
  try {
    const path = await invoke("choose_source", { prompt: t("dialog.chooseSource") });
    if (path) await loadSource(path);
  } catch (error) {
    showToast(localizeBackendError(error), 7000);
  }
}

async function loadSource(path) {
  if (!path || state.busy) return;
  const kind = path.toLowerCase().endsWith(".tex")
    ? "tex"
    : path.toLowerCase().endsWith(".pdf")
      ? "pdf"
      : null;
  if (!kind) {
    showToast(t("source.invalid"));
    return;
  }

  state.path = path;
  state.kind = kind;
  state.info = null;
  state.fontDecision = null;
  state.texFontDecision = null;
  state.pendingConversion = false;
  configureMode(kind);
  elements.empty.hidden = true;
  elements.file.hidden = false;
  elements.clear.hidden = false;
  elements.sourceKind.textContent = kind.toUpperCase();
  elements.fileName.textContent = path.split(/[\\/]/).pop();
  elements.fileMeta.textContent = t(kind === "tex" ? "file.texPreparing" : "file.pdfPreparing");
  elements.textCount.textContent = "—";
  elements.pageCount.textContent = "—";
  elements.fontCount.textContent = "—";
  elements.fontList.textContent = "";
  setBadge("inspection.analyzing", true);
  elements.convert.disabled = true;
  elements.warning.hidden = true;
  elements.progress.hidden = true;

  if (kind === "tex") {
    try {
      // Re-probe rather than reusing the cached status: a dependency installed
      // while TeXKey was open would otherwise stay invisible until relaunch,
      // leaving the convert button disabled with no explanation.
      const [env, info] = await Promise.all([
        invoke("environment_status"),
        invoke("inspect_tex", { path }),
      ]);
      state.env = env;
      state.info = info;
      renderEnvironmentStatus();
      renderSourceInfo();
      updateConvertAvailability();
    } catch (error) {
      clearSelection();
      showToast(localizeBackendError(error), 7000);
    }
    return;
  }

  try {
    const info = await invoke("inspect_pdf", { path });
    state.info = info;
    renderSourceInfo();
    updateConvertAvailability();
  } catch (error) {
    clearSelection();
    showToast(localizeBackendError(error), 7000);
  }
}

function clearSelection() {
  if (state.busy) return;
  state.path = null;
  state.info = null;
  state.kind = null;
  state.fontDecision = null;
  state.texFontDecision = null;
  state.pendingConversion = false;
  elements.empty.hidden = false;
  elements.file.hidden = true;
  elements.clear.hidden = true;
  elements.progress.hidden = true;
  elements.progressBar.style.width = "0";
  configureMode(null);
  updateConvertAvailability();
}

async function chooseOutput() {
  if (!invoke) {
    showToast(t("toast.tauriOnly"));
    return;
  }
  try {
    const dir = await invoke("choose_output_folder", { prompt: t("dialog.chooseOutput") });
    if (dir) {
      state.outputDir = dir;
      elements.outputLabel.textContent = dir.split(/[\\/]/).filter(Boolean).pop() || dir;
      elements.outputLabel.title = dir;
    }
  } catch (error) {
    showToast(localizeBackendError(error), 7000);
  }
}

function hasMissingPdfFonts() {
  return state.kind === "pdf" && Boolean(state.info?.missingFonts?.length);
}

function openFontRiskDialog(forConversion = false) {
  if (!hasMissingPdfFonts()) return false;
  const extractable = state.info.extractableFonts || [];
  state.pendingConversion = forConversion;
  elements.extractableFontList.textContent = extractable.length
    ? extractable
        .map((font) => `${font.name}${font.subset ? t("fontDialog.subsetSuffix") : ""}`)
        .join(" · ")
    : t("fontDialog.none");
  elements.extractFontButton.disabled = extractable.length === 0;
  elements.fontDialog.showModal();
  return true;
}

function hasMissingTexFonts() {
  return state.kind === "tex" && Boolean(state.info?.missingFonts?.length);
}

function openTexFontDialog(forConversion = false) {
  if (!hasMissingTexFonts()) return false;
  state.pendingConversion = forConversion;
  elements.texMissingFontList.textContent = state.info.missingFonts.join(" · ");
  elements.texFontDialog.showModal();
  return true;
}

async function startConversion() {
  if (!canConvert()) return;
  const mode = elements.mode.value;
  const exactImage = mode === "exactImage";
  const texDirect = mode === "texDirect";
  if (texDirect && state.texFontDecision === null && openTexFontDialog(true)) {
    return;
  }
  if (!exactImage && !texDirect && state.fontDecision === null && openFontRiskDialog(true)) {
    return;
  }

  state.busy = true;
  updateConvertAvailability();
  elements.clear.disabled = true;
  elements.progress.hidden = false;
  elements.progressBar.style.width = "15%";
  elements.progressTitle.textContent = t(
    texDirect ? "progress.texTitle" : exactImage ? "progress.exactTitle" : "progress.editableTitle",
  );
  elements.progressDetail.textContent = t(
    texDirect ? "progress.texDetail" : exactImage ? "progress.exactDetail" : "progress.editableDetail",
  );

  const timer = setTimeout(() => {
    elements.progressBar.style.width = "62%";
    elements.progressTitle.textContent = t("progress.keynoteTitle");
    elements.progressDetail.textContent = t("progress.keynoteDetail");
  }, 900);

  try {
    const result = await invoke("convert_pdf", {
      request: {
        inputPath: state.path,
        outputDirectory: state.outputDir,
        openAfter: $("open-toggle").checked,
        mode,
        extractEmbeddedFonts: state.fontDecision === "extract",
      },
    });
    clearTimeout(timer);
    elements.progressBar.style.width = "100%";
    elements.progressTitle.textContent = t("progress.complete");
    elements.progressDetail.textContent = result.outputPath;
    const resultKey = texDirect
      ? "result.texDirect"
      : exactImage
        ? "result.exactImage"
        : "result.editable";
    showToast(
      t("toast.converted", {
        pages: formatInteger(result.pages),
        kind: t(resultKey),
      }),
    );
    if (result.skippedFonts?.length) {
      setTimeout(
        () => showToast(t("toast.fontSkipped", { fonts: result.skippedFonts.join(", ") }), 6500),
        3900,
      );
    } else if (result.extractedFonts) {
      setTimeout(
        () => showToast(t("toast.fontExtracted", { count: formatInteger(result.extractedFonts) })),
        3900,
      );
    }
  } catch (error) {
    clearTimeout(timer);
    elements.progress.hidden = true;
    showToast(localizeBackendError(error), 7000);
  } finally {
    state.busy = false;
    elements.clear.disabled = false;
    updateConvertAvailability();
  }
}

function updateStatus(element, available, goodKey, badKey) {
  element.classList.toggle("bad", !available);
  element.querySelector("em").textContent = t(available ? goodKey : badKey);
}

function renderEnvironmentStatus() {
  if (!state.env) return;
  updateStatus(
    elements.keynoteStatus,
    state.env.keynoteInstalled,
    "status.ready",
    "status.required",
  );
  updateStatus(
    elements.pdfiumStatus,
    state.env.nativeEngineReady,
    "status.active",
    "status.unavailable",
  );
  updateStatus(
    elements.texStatus,
    state.env.texEngineReady,
    state.env.texEngineBundled ? "status.ready" : "status.external",
    "status.texMissing",
  );
  // Name the missing tools inline. Burying them in a title attribute meant the
  // one actionable detail was invisible unless the user thought to hover.
  const missing = state.env.missingTexTools || [];
  if (missing.length) {
    elements.texStatus.querySelector("em").textContent = missing.join(", ");
  }
  elements.texStatus.title = state.env.texEngineReady
    ? state.env.texEnginePaths.join("\n")
    : `${t("status.texMissing")}: ${missing.join(", ")}`;
}

// Dependencies can appear after launch; let the pill be re-probed on demand.
async function recheckEnvironment() {
  if (!invoke) return;
  try {
    state.env = await invoke("environment_status");
    renderEnvironmentStatus();
    updateConvertAvailability();
  } catch (error) {
    showToast(localizeBackendError(error), 7000);
  }
}

function refreshLocalizedUi() {
  const selectedMode = elements.mode.value;
  applyTranslations();
  elements.language.value = getLanguagePreference();
  configureMode(state.kind);
  if ([...elements.mode.options].some((item) => item.value === selectedMode)) {
    elements.mode.value = selectedMode;
    if (state.kind === "pdf") {
      elements.modeHelp.textContent = t(
        selectedMode === "exactImage" ? "method.exactImageHelp" : "method.editableHelp",
      );
    }
  }
  if (state.info) {
    renderSourceInfo();
  } else if (state.kind) {
    elements.fileMeta.textContent = t(
      state.kind === "tex" ? "file.texPreparing" : "file.pdfPreparing",
    );
  }
  if (state.outputDir) {
    elements.outputLabel.textContent =
      state.outputDir.split(/[\\/]/).filter(Boolean).pop() || state.outputDir;
  }
  renderEnvironmentStatus();
}

// Read the version from the bundle rather than hardcoding it in the markup,
// where it silently drifts behind each release.
async function showAppVersion() {
  const label = $("app-version");
  if (!label) return;
  try {
    const version = await window.__TAURI__?.app?.getVersion?.();
    label.textContent = version ? `TeXKey · v${version}` : "TeXKey";
  } catch {
    label.textContent = "TeXKey";
  }
}

async function boot() {
  applyTranslations();
  elements.language.value = getLanguagePreference();
  setAppearance(savedAppearance(), false);
  configureMode(null);
  showAppVersion();

  if (!invoke) return;

  try {
    const env = await invoke("environment_status");
    state.env = env;
    renderEnvironmentStatus();
  } catch (error) {
    showToast(localizeBackendError(error), 7000);
  }

  const current = window.__TAURI__?.webview?.getCurrentWebview?.();
  if (current?.onDragDropEvent) {
    await current.onDragDropEvent((event) => {
      const type = event.payload.type;
      elements.sourcePane.classList.toggle("dragging", type === "over");
      elements.drop.classList.toggle("dragging", type === "over");
      if (type === "drop") {
        const path = event.payload.paths?.find((item) => /\.(pdf|tex)$/i.test(item));
        if (path) loadSource(path);
        else showToast(t("source.invalid"));
      }
      if (type === "cancel") {
        elements.sourcePane.classList.remove("dragging");
        elements.drop.classList.remove("dragging");
      }
    });
  }
}

elements.drop.addEventListener("click", chooseSource);
$("replace-button").addEventListener("click", chooseSource);
elements.clear.addEventListener("click", clearSelection);
$("output-button").addEventListener("click", chooseOutput);
elements.convert.addEventListener("click", startConversion);
elements.texStatus.addEventListener("click", recheckEnvironment);
elements.texStatus.style.cursor = "pointer";
$("font-help-button").addEventListener("click", () => {
  const opened =
    state.kind === "tex" ? openTexFontDialog(false) : openFontRiskDialog(false);
  if (!opened) showToast(t("toast.fontHelp"), 6000);
});
elements.mode.addEventListener("change", (event) => {
  const exactImage = event.target.value === "exactImage";
  elements.modeHelp.textContent = t(exactImage ? "method.exactImageHelp" : "method.editableHelp");
  showToast(t(exactImage ? "toast.exactMode" : "toast.editableMode"));
});
document.querySelectorAll("[data-theme-value]").forEach((button) => {
  button.addEventListener("click", () => setAppearance(button.dataset.themeValue));
});
document.querySelector(".titlebar").addEventListener("mousedown", async (event) => {
  if (
    event.button !== 0
    || event.target.closest("button, select, input, a, .title-actions")
    || !appWindow?.startDragging
  ) {
    return;
  }
  event.preventDefault();
  try {
    await appWindow.startDragging();
  } catch {
    // The data-tauri-drag-region attributes remain as the native fallback.
  }
});
elements.language.addEventListener("change", (event) => {
  setLanguagePreference(event.target.value);
  refreshLocalizedUi();
});
elements.fontDialog.addEventListener("close", () => {
  const shouldConvert = state.pendingConversion;
  state.pendingConversion = false;
  if (elements.fontDialog.returnValue === "extract") {
    state.fontDecision = "extract";
  } else if (elements.fontDialog.returnValue === "skip") {
    state.fontDecision = "skip";
  } else {
    return;
  }
  if (shouldConvert) startConversion();
});
elements.texFontDialog.addEventListener("close", () => {
  const shouldConvert = state.pendingConversion;
  state.pendingConversion = false;
  if (elements.texFontDialog.returnValue !== "continue") return;
  state.texFontDecision = "continue";
  if (shouldConvert) startConversion();
});

boot();
