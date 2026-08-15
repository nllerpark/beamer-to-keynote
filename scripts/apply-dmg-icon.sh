#!/bin/sh
set -eu

project_dir=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
icon_dir="$project_dir/src-tauri/icons"
dmg_dir="$project_dir/src-tauri/target/aarch64-apple-darwin/release/bundle/dmg"

for dmg_path in "$dmg_dir"/*.dmg; do
  [ -e "$dmg_path" ] || continue
  (
    cd "$icon_dir"
    xcrun Rez -append dmg-icon.r -o "$dmg_path"
  )
  xcrun SetFile -a C "$dmg_path"
done
