#!/bin/sh
# Ad-hoc sign an unsigned build so it can still ask for Keynote automation.
#
# `tauri build --no-sign` leaves the bundle linker-signed with no entitlements.
# Keynote automation then fails without ever prompting, because
# com.apple.security.automation.apple-events is absent and TCC has no stable
# identity to attach a grant to. Ad-hoc signing needs no certificate and
# restores the entitlement, which is the difference between a build that can
# request permission and one that silently cannot.
#
# This is a fallback for local and unsigned builds. A Developer ID signature
# remains the only way to get a durable TCC grant and pass notarisation.
#
# Usage: sh scripts/adhoc-sign.sh [path/to/TeXKey.app]

set -eu

project_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
app_path=${1:-"$project_root/src-tauri/target/aarch64-apple-darwin/release/bundle/macos/TeXKey.app"}
entitlements="$project_root/src-tauri/Entitlements.plist"

if [ ! -d "$app_path" ]; then
  echo "error: app bundle not found: $app_path" >&2
  exit 1
fi
if [ ! -f "$entitlements" ]; then
  echo "error: entitlements not found: $entitlements" >&2
  exit 1
fi

# Sign nested code first, then the bundle, so the outer signature seals them.
find "$app_path/Contents" \( -name '*.dylib' -o -name '*.so' \) -type f 2>/dev/null |
  while IFS= read -r nested; do
    codesign --force --sign - --timestamp=none "$nested"
  done

for helper in "$app_path/Contents/MacOS/"*; do
  [ -f "$helper" ] || continue
  case "$helper" in
    */texkey) continue ;;  # signed as part of the bundle below
  esac
  codesign --force --sign - --timestamp=none "$helper"
done

codesign --force --sign - \
  --entitlements "$entitlements" \
  --options runtime \
  --timestamp=none \
  "$app_path"

codesign --verify --strict "$app_path"

if ! codesign -d --entitlements - "$app_path" 2>&1 |
  grep -q "com.apple.security.automation.apple-events"; then
  echo "error: apple-events entitlement missing after signing" >&2
  exit 1
fi

echo "ad-hoc signed with automation entitlement: $app_path"
echo "note: TCC grants for ad-hoc signatures are keyed to the binary hash and"
echo "      are lost on every rebuild. Ship a Developer ID signed build."
