#!/bin/sh
set -eu

project_dir=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
source_ref=${1:-HEAD}
version=$(node -p "require('$project_dir/package.json').version")
archive_dir="$project_dir/src-tauri/target/aarch64-apple-darwin/release/bundle/source"
archive_name="TeXKey_${version}_source.tar.gz"
archive_path="$archive_dir/$archive_name"

git -C "$project_dir" rev-parse --verify "${source_ref}^{commit}" >/dev/null

if [ "$source_ref" = "HEAD" ] &&
  [ -n "$(git -C "$project_dir" status --porcelain --untracked-files=normal)" ]; then
  echo "error: create the release archive from a clean Git commit" >&2
  exit 1
fi

mkdir -p "$archive_dir"

git -C "$project_dir" archive \
  --format=tar.gz \
  --prefix="TeXKey-${version}/" \
  --output="$archive_path" \
  "$source_ref"

(
  cd "$archive_dir"
  shasum -a 256 "$archive_name" > "${archive_name}.sha256"
)

echo "source archive: $archive_path"
echo "checksum: ${archive_path}.sha256"
