#!/bin/sh
# Refresh the bundled UltiMed v2 fix rules from a local clone of the rules repo
# (https://github.com/UltiMed-Org/UltiMed-ASR-FR-v2-regex-fixes).
#
#   ./sync-regex-fixes.sh [path/to/UltiMed-ASR-FR-v2-regex-fixes]
#
# The clone must be checked out at a release tag (e.g. v2.0.0): every file is
# checked against that tag's SHA256SUMS before it is copied.
#
# Copies its rules/*_fix_rules.jsonl and rules/confusion_groups.jsonl byte for byte and records each file's commit and
# sha256 in regex-fixes/PROVENANCE, then reminds you to run the tests.
# Written with Claude Code.
set -eu

here="$(cd "$(dirname "$0")" && pwd)"
repo="${1:-$here/../../../ASR_parakeet_finetuning_ultimed_nemo/UltiMed-ASR-FR-v2-regex-fixes}"
src="$repo/rules"
dst="$here/regex-fixes"
# Applied in this order (keep in sync with BUNDLED_SOURCES in
# src/medical/drug_rules.rs): the hand-written file runs last, so its spelling
# conventions also respell what a learned rule wrote.
files="drug_fix_rules.jsonl dictionary_fix_rules.jsonl parhaf_fix_rules.jsonl parrot_fix_rules.jsonl acronyms_fix_rules.jsonl manual_fix_rules.jsonl"
# Not a rule file: read by the decoder's second pass (src/engine/confusion.rs).
files="$files confusion_groups.jsonl"

[ -d "$src" ] || { echo "ERROR: $src not found" >&2; exit 1; }
# Sync from a release tag: its SHA256SUMS is checked below (this catches a
# corrupted or mismatched checkout; PROVENANCE pinned in this repo plus the
# signed online sets are what protect against tampering).
tag="$(git -C "$repo" describe --tags --exact-match 2>/dev/null || true)"
[ -n "$tag" ] || { echo "ERROR: check out a release tag of the rules repo first" >&2; exit 1; }
[ -f "$repo/SHA256SUMS" ] || { echo "ERROR: $repo/SHA256SUMS not found" >&2; exit 1; }
mkdir -p "$dst"
# Drop files of an older layout (v1 had term_fix_rules.jsonl).
rm -f "$dst"/*_fix_rules.jsonl
echo "# tag $tag" > "$dst/PROVENANCE"
for f in $files; do
    expected="$(grep " rules/$f\$" "$repo/SHA256SUMS" | cut -d' ' -f1)"
    actual="$(sha256sum "$src/$f" | cut -d' ' -f1)"
    [ -n "$expected" ] && [ "$expected" = "$actual" ] || { echo "ERROR: rules/$f does not match SHA256SUMS" >&2; exit 1; }
    cp "$src/$f" "$dst/$f"
    commit="$(git -C "$repo" log -1 --format=%H -- "rules/$f" 2>/dev/null || echo unknown)"
    if [ -n "$(git -C "$repo" status --porcelain -- "rules/$f" 2>/dev/null)" ]; then
        commit="$commit (plus uncommitted changes)"
    fi
    sha="$(sha256sum "$dst/$f" | cut -d' ' -f1)"
    echo "$f $sha $commit" >> "$dst/PROVENANCE"
done
cat "$dst/PROVENANCE"
echo "Now run: cd src-tauri && cargo test --lib medical"
