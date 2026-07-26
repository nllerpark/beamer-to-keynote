# TeXKey — Beamer and PDF to Keynote

TeXKey is a native macOS application for translating Beamer sources and
existing PDF slide decks into Apple Keynote documents. Its interface is built
with Tauri, while document analysis and conversion are implemented in Rust.
The standard distribution carries its rendering engines with it and requires
no Python, Node.js, TeX Live, or auxiliary PDF installation at run time.

## Direct Beamer rendering

![TeXKey rendering a Beamer source directly into Keynote](docs/texkey-demo.gif)

When given a Beamer `.tex` source, TeXKey typesets the presentation to XDV and
constructs vector Keynote slides without producing an intermediate PDF.

For existing PDFs, TeXKey offers two complementary interpretations:

- **Editable text** retains the non-textual page as a vector PDF layer and
  reconstructs textual material as native Keynote text objects.
- **Exact appearance** places each complete PDF page as a single sharp image
  layer, avoiding font substitution and object displacement.

The converter preserves character-level Unicode, typeface, size, colour,
position, and rotation. Mathematical characters represented by UTF-16
surrogate pairs are retained. Before conversion, TeXKey examines the source,
the fonts available to macOS, and the local Keynote environment.

The interface follows the user’s locale in English or Korean. Its appearance
may follow macOS—the default—or be fixed to light or dark mode.

## Font handling for editable PDFs

Keynote does not expose a public API for embedding editable fonts inside a
`.key` document. TeXKey therefore includes the OFL-licensed Libertinus family
and installs it in the user font directory when required. The original
non-textual PDF layer remains in the Keynote document, preserving the visual
ground beneath reconstructed text.

If a PDF requires a font that is absent from the current Mac, TeXKey checks
whether the PDF contains an actual embedded font program. Before an editable
conversion, it presents the missing fonts and asks whether they should be
extracted. The default action is to continue without extraction. Extraction
occurs only after explicit user consent.

The consent notice states the following risks:

- Inclusion in a PDF does not grant permission to install or reuse a font.
- A subset font may contain only the glyphs used by the PDF; newly entered
  characters may therefore fall back to another font.
- Untrusted font files may present a security risk. Extracted fonts are
  installed for the user account and remain after conversion.
- TeXKey installs only embedded OpenType or TrueType programs. Non-embedded
  fonts, Type 3 fonts, and unsupported formats are not extracted.

The exact-appearance mode does not require font extraction or installation.

## Distribution profiles

TeXKey is issued in two macOS profiles.

### TeXKey

The complete profile includes Tectonic, dvisvgm, and an offline TeX resource
set. It is intended as the self-contained distribution: Beamer and PDF
conversion work without an external typesetting installation.

The TeXKey source code is licensed under BSD 3-Clause. The bundled dvisvgm
executable remains licensed under GPLv3 or later. A distribution containing
dvisvgm must comply with the GPLv3-or-later terms applicable to that component.
Other bundled components retain their respective upstream licences.

### TeXKey BSD

The BSD profile omits the bundled TeX executables and offline TeX resource
payload. PDF conversion remains self-contained. Direct Beamer conversion uses
`tectonic` and `dvisvgm` installed separately on the Mac.

The simplest supported installation is:

```sh
brew install tectonic dvisvgm
```

Homebrew’s dvisvgm formula installs its TeX Live dependency. If a complete
MacTeX installation already provides `/Library/TeX/texbin/dvisvgm`, only
Tectonic is additionally required:

```sh
brew install tectonic
```

The first external Tectonic run may retrieve its support bundle over the
network. TeXKey searches the application bundle, `PATH`,
`/opt/homebrew/bin`, `/usr/local/bin`, and `/Library/TeX/texbin`.

## System requirements

- Apple Silicon Mac running macOS 12 or later
- Apple Keynote
- Rust, Node.js, and Xcode Command Line Tools for development builds
- No additional run-time installation for the complete TeXKey profile
- `tectonic` and `dvisvgm` for direct Beamer conversion in TeXKey BSD

## Building from source

A source build requires Git, the stable Rust toolchain, a current Node.js LTS
release, npm, and Xcode Command Line Tools. The checked-in PDFium framework and
the complete-profile TeX executables presently target Apple Silicon.

Begin with a clean checkout:

```sh
git clone https://github.com/nllerpark/beamer-to-keynote.git
cd beamer-to-keynote
npm ci
```

Run the development application:

```sh
npm run dev
```

Create an unsigned complete distribution:

```sh
npm run build:unsigned
```

Create an unsigned BSD distribution:

```sh
npm run build:bsd:unsigned
```

The resulting application and DMG are written below
`src-tauri/target/aarch64-apple-darwin/release/bundle/`. The BSD build itself
does not require a TeX installation, but direct Beamer conversion in the
resulting application requires the external tools described above.

## Source release archive

Every TeXKey release should publish the source archive generated from the same
Git commit as its binary artifacts:

```sh
npm run package:source
```

This produces `TeXKey_<version>_source.tar.gz` and its SHA-256 checksum below
`src-tauri/target/aarch64-apple-darwin/release/bundle/source/`. For a public
release, run the command from the tagged, clean commit and attach both files to
the release. Hosting services may also provide their own automatic archives
for the tag.

This archive contains the TeXKey repository at the selected commit. It is not
a substitute for the corresponding source required for bundled GPL software.
The complete-profile release must also retain the dvisvgm source information
recorded in `src-tauri/resources/tex-engines/DVISVGM-SOURCE.md`.

## Signed macOS builds

Build the complete profile:

```sh
npm run build
```

Build the external-tool BSD profile:

```sh
npm run build:bsd
```

Artifacts are written below
`src-tauri/target/aarch64-apple-darwin/release/bundle/`. Both profiles use the
configured ITRIX Developer ID identity to sign the application and DMG. These
build commands do not submit the result for Apple notarization.

Notarization remains available as an optional release operation:

```sh
xcrun notarytool store-credentials TeXKey \
  --apple-id "APPLE_ID" \
  --team-id "D5UUNR2X89"

APPLE_NOTARY_PROFILE=TeXKey npm run release:macos
```

The release script builds and signs the complete profile, creates the source
archive and checksum, submits the DMG, waits for Apple’s response, staples the
ticket, and verifies the resulting artifact.

## Repository structure

- `src/` — the Stitch-informed macOS interface and locale resources
- `src-tauri/src/pdfium_engine.rs` — character-level PDF reconstruction
- `src-tauri/src/tex_engine.rs` — direct XDV-to-vector Beamer rendering
- `src-tauri/resources/pdfium/` — the bundled PDFium engine and notices
- `src-tauri/resources/fonts/` — the bundled Libertinus family
- `keynote_editable_import.applescript` — native Keynote object construction

## Copyright and licences

Copyright © 2026 Park Junhu (`nller.park@snu.ac.kr`).

TeXKey source code is distributed under the
[BSD 3-Clause License](LICENSE). PDFium, Tectonic, dvisvgm, TeX resources, and
fonts retain their own licences. See the notices included with each bundled
component. A binary distribution that includes dvisvgm is subject to the
GPLv3-or-later conditions applicable to dvisvgm.
