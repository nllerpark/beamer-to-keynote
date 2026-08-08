#!/bin/sh
set -eu

project_dir=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
profile=${APPLE_NOTARY_PROFILE:-${1:-}}
signing_identity=${APPLE_SIGNING_IDENTITY:-Developer ID Application: ITRIX Co., Ltd. (D5UUNR2X89)}

if [ -z "$profile" ]; then
  echo "error: APPLE_NOTARY_PROFILE is required" >&2
  echo "store credentials with xcrun notarytool store-credentials, then retry" >&2
  exit 1
fi

preflight_failed=0
if ! security find-identity -v -p codesigning |
  grep -F "\"$signing_identity\"" >/dev/null; then
  echo "error: code-signing identity is not installed: $signing_identity" >&2
  echo "install the Developer ID Application certificate and its private key" >&2
  preflight_failed=1
fi
if ! xcrun notarytool history --keychain-profile "$profile" >/dev/null 2>&1; then
  echo "error: notarytool keychain profile is unavailable or invalid: $profile" >&2
  echo "store credentials with xcrun notarytool store-credentials $profile" >&2
  preflight_failed=1
fi
if [ "$preflight_failed" -ne 0 ]; then
  exit 1
fi

cd "$project_dir"

version=$(node -p "require('./package.json').version")
bundle_dir="$project_dir/src-tauri/target/aarch64-apple-darwin/release/bundle"
release_dir="$project_dir/src-tauri/target/release-artifacts"
rm -rf "$release_dir"
mkdir -p "$release_dir"

# Sign, notarise, staple, and verify one DMG.
notarise_dmg() {
  dmg_path=$1
  app_path=$2

  codesign --verify --deep --strict --verbose=2 "$app_path"
  codesign --verify --verbose=2 "$dmg_path"

  xcrun notarytool submit "$dmg_path" \
    --keychain-profile "$profile" \
    --wait

  xcrun stapler staple "$dmg_path"
  xcrun stapler validate "$dmg_path"
  codesign --verify --verbose=2 "$dmg_path"
  spctl --assess --type open --context context:primary-signature --verbose=4 "$dmg_path"
}

# --- BSD build: redistributes no GPL-licensed program -----------------------
echo "==> building BSD variant"
npm run build
npm run package:source

app_path="$bundle_dir/macos/TeXKey.app"
dmg_path="$bundle_dir/dmg/TeXKey_${version}_aarch64.dmg"
source_path="$bundle_dir/source/TeXKey_${version}_source.tar.gz"

if [ ! -d "$app_path" ] || [ ! -f "$dmg_path" ] || [ ! -f "$source_path" ]; then
  echo "error: expected TeXKey app, DMG, and source archive were not produced" >&2
  exit 1
fi

notarise_dmg "$dmg_path" "$app_path"
mv "$dmg_path" "$release_dir/TeXKey_${version}_aarch64_bsd.dmg"
cp "$source_path" "$release_dir/TeXKey_${version}_source.tar.gz"

# --- GPL build: bundles dvisvgm and Ghostscript -----------------------------
# Carries GPLv3/AGPLv3 source-conveyance obligations; see the notices under
# src-tauri/resources/tex-engines and src-tauri/resources/ghostscript.
echo "==> building GPL variant"
npm run build:gpl

if [ ! -d "$app_path" ] || [ ! -f "$dmg_path" ]; then
  echo "error: expected GPL TeXKey app and DMG were not produced" >&2
  exit 1
fi

# The vendored Ghostscript closure is signed as part of the bundle; verify each
# library carries a valid signature before the DMG is submitted.
for library in "$app_path/Contents/Resources/ghostscript/lib/"*.dylib; do
  [ -f "$library" ] || continue
  codesign --verify --strict --verbose=1 "$library"
done

notarise_dmg "$dmg_path" "$app_path"
mv "$dmg_path" "$release_dir/TeXKey_${version}_aarch64_gpl.dmg"

echo
echo "release artifacts ready in: $release_dir"
ls -1 "$release_dir"
