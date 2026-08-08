#!/bin/sh
# Vendor the Ghostscript shared library into the GPL build.
#
# Copies libgs and its full non-OS dependency closure into
# src-tauri/resources/ghostscript/lib, rewrites every install name to
# @loader_path so the copies resolve inside the app bundle, and records the
# provenance needed to satisfy AGPL/GPL source conveyance.
#
# The BSD build does not use any of this; it loads the libgs installed by the
# user via Homebrew. Run this only when producing the GPL release.
#
# Usage: sh scripts/vendor-ghostscript.sh [--source <libgs.dylib>]

set -eu

project_root=$(cd "$(dirname "$0")/.." && pwd)
destination="$project_root/src-tauri/resources/ghostscript/lib"
manifest="$project_root/src-tauri/resources/ghostscript/GHOSTSCRIPT-SOURCE.md"

source_library=""
while [ $# -gt 0 ]; do
  case "$1" in
    --source) source_library=${2:?--source needs a path}; shift 2 ;;
    *) echo "unknown argument: $1" >&2; exit 2 ;;
  esac
done

if [ -z "$source_library" ]; then
  for candidate in \
    /opt/homebrew/opt/ghostscript/lib/libgs.dylib \
    /usr/local/opt/ghostscript/lib/libgs.dylib
  do
    [ -f "$candidate" ] && { source_library=$candidate; break; }
  done
fi

if [ -z "$source_library" ] || [ ! -f "$source_library" ]; then
  echo "Ghostscript shared library not found. Install it with 'brew install ghostscript'," >&2
  echo "or pass an explicit path with --source." >&2
  exit 1
fi

echo "Vendoring Ghostscript from: $source_library"
rm -rf "$destination"
mkdir -p "$destination"

# Resolve the transitive closure of non-OS dependencies, copy each one in, and
# rewrite install names. /usr/lib and /System are provided by macOS and are
# deliberately left as absolute references.
python3 - "$source_library" "$destination" <<'PY'
import os, re, shutil, subprocess, sys

source, destination = sys.argv[1], sys.argv[2]

def dependencies(path):
    out = subprocess.run(["otool", "-L", path], capture_output=True, text=True).stdout
    return [m.group(1) for m in
            (re.match(r"\s+(\S+)\s+\(compat", line) for line in out.splitlines()[1:]) if m]

def is_system(path):
    return path.startswith("/usr/lib") or path.startswith("/System")

closure = {}
def walk(path):
    real = os.path.realpath(path)
    if real in closure or is_system(real) or not os.path.exists(real):
        return
    closure[real] = os.path.basename(real)
    for dependency in dependencies(real):
        if not is_system(dependency):
            walk(dependency)

walk(source)

for real, name in closure.items():
    shutil.copy2(real, os.path.join(destination, name))
    os.chmod(os.path.join(destination, name), 0o755)

# Rewrite the identity and every non-OS dependency reference to @loader_path.
for name in closure.values():
    target = os.path.join(destination, name)
    subprocess.run(["install_name_tool", "-id", f"@loader_path/{name}", target], check=True)
    for dependency in dependencies(target):
        if is_system(dependency):
            continue
        basename = os.path.basename(os.path.realpath(dependency))
        if basename in closure.values():
            subprocess.run(
                ["install_name_tool", "-change", dependency, f"@loader_path/{basename}", target],
                check=True,
            )

# libgs.dylib is the name the application dlopens; keep it as a real file so the
# bundle does not depend on symlink preservation.
versioned = os.path.basename(os.path.realpath(source))
if versioned != "libgs.dylib":
    shutil.copy2(os.path.join(destination, versioned), os.path.join(destination, "libgs.dylib"))
    subprocess.run(
        ["install_name_tool", "-id", "@loader_path/libgs.dylib",
         os.path.join(destination, "libgs.dylib")],
        check=True,
    )

# install_name_tool invalidates the existing code signature, and arm64 refuses
# to load a dylib whose signature does not match its contents — the loading
# process is killed outright. Re-sign ad hoc; the release build replaces these
# with Developer ID signatures when it signs the bundle.
for name in sorted(set(closure.values()) | {"libgs.dylib"}):
    target = os.path.join(destination, name)
    if not os.path.exists(target):
        continue
    subprocess.run(
        ["codesign", "--force", "--sign", "-", "--timestamp=none", target],
        check=True, capture_output=True,
    )
    subprocess.run(["codesign", "--verify", "--strict", target], check=True)

print(f"vendored {len(closure)} libraries")
for real in sorted(closure):
    print(f"  {os.path.basename(real)}")
PY

total=$(du -sh "$destination" | awk '{print $1}')
count=$(find "$destination" -name '*.dylib' | wc -l | tr -d ' ')

# Record provenance for source conveyance. Ghostscript is AGPL-3.0-or-later and
# its dependency closure carries its own licences; both must be conveyed.
{
  printf '# Ghostscript source availability\n\n'
  printf 'The GPL build of TeXKey bundles the Ghostscript shared library and its\n'
  printf 'dependency closure, vendored by `scripts/vendor-ghostscript.sh`.\n\n'
  printf -- '- vendored from: `%s`\n' "$source_library"
  printf -- '- libraries: %s\n' "$count"
  printf -- '- total size: %s\n' "$total"
  printf -- '- vendored on: %s\n\n' "$(date -u '+%Y-%m-%d')"
  printf 'Ghostscript is licensed under the GNU Affero General Public License\n'
  printf 'version 3 or, at the recipient'"'"'s option, any later version. Complete\n'
  printf 'corresponding source is available from:\n\n'
  printf -- '- https://github.com/ArtifexSoftware/ghostpdl\n'
  printf -- '- https://ghostscript.com/releases/gsdnld.html\n\n'
  printf 'These source locations must remain available alongside any public download\n'
  printf 'of the TeXKey GPL distribution. A redistributor is responsible for\n'
  printf 'satisfying the source-conveyance requirements of AGPLv3 section 6.\n\n'
  printf '## Bundled libraries\n\n'
  printf 'Each carries its own licence and must be conveyed with its own notice.\n\n'
  for library in "$destination"/*.dylib; do
    printf -- '- `%s` — sha256 `%s`\n' \
      "$(basename "$library")" "$(shasum -a 256 "$library" | awk '{print $1}')"
  done
} > "$manifest"

echo
echo "Vendored $count libraries ($total) into:"
echo "  $destination"
echo "Provenance recorded in:"
echo "  $manifest"
echo
echo "Verify the closure resolves with no absolute Homebrew references:"
echo "  otool -L $destination/libgs.dylib"
