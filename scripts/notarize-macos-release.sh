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
npm run build
npm run package:source

version=$(node -p "require('./package.json').version")
bundle_dir="$project_dir/src-tauri/target/aarch64-apple-darwin/release/bundle"
app_path="$bundle_dir/macos/TeXKey.app"
dmg_path="$bundle_dir/dmg/TeXKey_${version}_aarch64.dmg"
source_path="$bundle_dir/source/TeXKey_${version}_source.tar.gz"

if [ ! -d "$app_path" ] || [ ! -f "$dmg_path" ] || [ ! -f "$source_path" ]; then
  echo "error: expected TeXKey app, DMG, and source archive were not produced" >&2
  exit 1
fi

codesign --verify --deep --strict --verbose=2 "$app_path"
codesign --verify --verbose=2 "$dmg_path"

xcrun notarytool submit "$dmg_path" \
  --keychain-profile "$profile" \
  --wait

xcrun stapler staple "$dmg_path"
xcrun stapler validate "$dmg_path"
codesign --verify --verbose=2 "$dmg_path"
spctl --assess --type open --context context:primary-signature --verbose=4 "$dmg_path"

echo "release ready: $dmg_path"
echo "source ready: $source_path"
