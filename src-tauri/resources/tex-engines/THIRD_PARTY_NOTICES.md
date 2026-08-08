# Bundled TeX rendering engines

These components are present only in the **GPL build** of TeXKey. The BSD
build redistributes none of them and invokes the copies installed by the user.

## dvisvgm 3.6

Copyright Martin Gieseking and contributors.

Licensed under GNU General Public License version 3 or later. Source for the
bundled version and the corresponding TeX Live 2026 sources are recorded in
`DVISVGM-SOURCE.md`. The complete GPL text is included in `GPL-3.0.txt`.

## Ghostscript

Copyright Artifex Software, Inc.

Licensed under the GNU Affero General Public License version 3 or later.
Binary provenance, the vendored dependency closure, and corresponding-source
locations are recorded in `../ghostscript/GHOSTSCRIPT-SOURCE.md`. The complete
AGPL text is included in `../ghostscript/AGPL-3.0.txt`.

Ghostscript is loaded by dvisvgm only to convert EPS and PostScript specials.
Documents containing none of these render identically without it.

## Ghostscript dependency closure

The Ghostscript shared library is vendored together with the libraries it
links against, listed with checksums in `../ghostscript/GHOSTSCRIPT-SOURCE.md`.
Each carries its own licence, including but not limited to:

- freetype — FreeType License or GPLv2
- libpng — PNG Reference Library License
- libtiff, libwebp, libarchive, leptonica — BSD-style licences
- jpeg-turbo — BSD-style and IJG licences
- little-cms2 — MIT
- openjpeg — BSD 2-Clause
- jbig2dec — AGPLv3
- tesseract — Apache-2.0
- libidn — LGPLv2.1 or later
- zstd — BSD or GPLv2; lz4 — BSD; xz/liblzma — public domain or GPL

A redistributor is responsible for conveying each licence and, where required,
the corresponding source.

## Relationship to TeXKey

The executables are separate helper programs invoked by TeXKey, and the
Ghostscript library is loaded by dvisvgm rather than by TeXKey. TeXKey does not
link against their program code, and its own source remains BSD-3-Clause.
