#!/bin/sh
set -eu

project_dir=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
target=aarch64-apple-darwin
bundle_dir="$project_dir/src-tauri/target/$target/release/bundle"
app_path="$bundle_dir/macos/TeXKey.app"
dmg_dir="$bundle_dir/dmg"
version=$(node -p "require('$project_dir/package.json').version")
dmg_path="$dmg_dir/TeXKey_${version}_aarch64.dmg"

cd "$project_dir"
./node_modules/.bin/tauri build --target "$target" --no-sign \
  --config src-tauri/tauri.bsd.conf.json

# `--no-sign` avoids Developer ID signing, but Apple Silicon macOS still
# requires a Mach-O code-signature region to execute an app.  Sign nested
# libraries first, then apply only an ad-hoc signature: it has no certificate,
# Team ID, notarization, or Hardened Runtime, so PDFium is not subject to
# library Team-ID validation.
codesign --force --sign - "$app_path/Contents/Frameworks/libpdfium.dylib"
find "$app_path" -type f -exec file {} + |
  sed -n 's/:.*Mach-O.*$/&/p' |
  cut -d: -f1 |
  while IFS= read -r binary_path; do
    codesign --force --sign - "$binary_path"
  done

rm -f "$dmg_path"
hdiutil create -volname TeXKey -srcfolder "$app_path" -format UDZO "$dmg_path"
sh scripts/apply-dmg-icon.sh
