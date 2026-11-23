#!/usr/bin/env bash
# Plumed fork: fetch Grammalecte 2.3.0 (French grammar checker, GPL v3+, https://grammalecte.net),
# the JavaScript build of its Firefox extension, into resources/grammalecte/, where the grammar step
# (src-tauri/src/grammar/) loads it into an embedded JavaScript engine. No Python is shipped.
#
# The extension package is pinned by its SHA-256. Only the files listed in
# src-tauri/grammar-data/grammalecte-files.txt are kept (the engine, the rules and the main
# dictionary; not the thesaurus, the tests nor the other dictionaries), plus Grammalecte's license.
#
# Usage:
#   scripts/fetch-grammalecte.sh                       # download from addons.mozilla.org (used by CI)
#   scripts/fetch-grammalecte.sh path/to/grammalecte_fr-2.3.0.xpi   # use a local copy
#
# Set up with Claude Code.
set -euo pipefail

URL="https://addons.mozilla.org/firefox/downloads/file/4643560/grammalecte_fr-2.3.0.xpi"
SHA256="443030004dbf62ecd5fc42f7b96872b96c9e3765f1cba4704a2a4170bb945256"
DEST="resources/grammalecte"
FILE_LIST="src-tauri/grammar-data/grammalecte-files.txt"
LICENSE="src-tauri/grammar-data/LICENSE-grammalecte.txt"

sha256() {
    if command -v sha256sum > /dev/null; then
        sha256sum "$1" | cut -d' ' -f1
    else
        shasum -a 256 "$1" | cut -d' ' -f1 # macOS
    fi
}

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT
XPI="$WORK/grammalecte.xpi"

if [ -n "${1:-}" ]; then
    echo "Copying $1"
    cp "$1" "$XPI"
else
    echo "Downloading $URL"
    curl --proto '=https' --fail --retry 3 -L -o "$XPI" "$URL"
fi
actual="$(sha256 "$XPI")"
if [ "$actual" != "$SHA256" ]; then
    echo "::error::SHA-256 mismatch for the Grammalecte package: expected $SHA256, got $actual" >&2
    exit 1
fi

# bash 3 compatible (macOS): no mapfile
FILES=()
while read -r kind path; do
    path="${path%$'\r'}" # the Windows runner checks the list out with CRLF line endings
    case "$kind" in
        js | data) FILES+=("$path") ;;
    esac
done < "$FILE_LIST"
# Extract the whole package, then copy only the listed files.
mkdir -p "$WORK/x"
if command -v unzip > /dev/null; then
    unzip -q -o "$XPI" -d "$WORK/x"
else
    python3 -c 'import sys, zipfile; zipfile.ZipFile(sys.argv[1]).extractall(sys.argv[2])' "$XPI" "$WORK/x"
fi

rm -rf "$DEST"
for f in "${FILES[@]}"; do
    mkdir -p "$DEST/$(dirname "$f")"
    cp "$WORK/x/$f" "$DEST/$f"
done
cp "$LICENSE" "$DEST/LICENSE.txt"
du -sh "$DEST"
