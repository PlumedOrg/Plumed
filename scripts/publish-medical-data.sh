#!/usr/bin/env bash
# Plumed fork (written with Claude Code): publish the medical data currently in
# src-tauri/medical-data/ as a signed online update (see the "Online updates"
# section of src-tauri/medical-data/SOURCE.md and docs/ARCHITECTURE.md).
#
# Builds manifest.json (sha256 and size of each file, the model id of
# MODEL_ID in src-tauri/src/medical/data_set/mod.rs, min_app_version = the
# package.json version), signs it with minisign, and uploads the files, the
# manifest and its signature as assets of the GitHub release "medical-data"
# of PlumedOrg/Plumed. That release is created as a prerelease that is never
# "latest": the app updater reads releases/latest/download/latest.json, which
# must keep pointing at the real app release.
#
# Usage (from anywhere in the repo):
#   scripts/publish-medical-data.sh <version> "<short French release notes>"
#   scripts/publish-medical-data.sh 2026.12.03 "Corrections de noms de médicaments"
# <version> is YYYY.MM.DD or YYYY.MM.DD.N and must be newer than the version
# the apps already have, or they will not offer it.
#
# Requirements: minisign, gh (logged in, allowed to edit releases), python,
# sha256sum or shasum.
#
# One-time key setup (keep the secret key off the repo and backed up):
#   minisign -G -p plumed-rules.pub -s ~/.minisign/plumed-rules.key
# then put the base64 line of plumed-rules.pub (the second line) in the
# PLUMED_RULES_PUBKEY repository secret, so release builds embed it.
#
# Environment:
#   PLUMED_RULES_KEY     secret key path (default ~/.minisign/plumed-rules.key)
#   PLUMED_RULES_PUBKEY  if set, the signature is checked against it before upload
set -eu

repo="PlumedOrg/Plumed"
tag="medical-data"

if [ "$#" -ne 2 ]; then
    sed -n '2,30p' "$0"
    exit 1
fi
version="$1"
notes="$2"

if ! printf '%s' "$version" | grep -Eq '^[0-9]{4}\.(0[1-9]|1[0-2])\.(0[1-9]|[12][0-9]|3[01])(\.[0-9]{1,4})?$'; then
    echo "Invalid version '$version' (expected YYYY.MM.DD or YYYY.MM.DD.N)" >&2
    exit 1
fi

root="$(git rev-parse --show-toplevel)"
data="$root/src-tauri/medical-data"
model="$(sed -n 's/^pub const MODEL_ID: &str = "\(.*\)";$/\1/p' "$root/src-tauri/src/medical/data_set/mod.rs")"
app_version="$(python -c 'import json, sys; print(json.load(open(sys.argv[1]))["version"])' "$root/package.json")"
if [ -z "$model" ]; then
    echo "MODEL_ID not found in src-tauri/src/medical/data_set/mod.rs" >&2
    exit 1
fi

# Same names, and for the rules the same order, as RULE_FILES, CONFUSION_FILE
# and PHRASE_FILE in src-tauri/src/medical/data_set/mod.rs.
files="
regex-fixes/drug_fix_rules.jsonl
regex-fixes/dictionary_fix_rules.jsonl
regex-fixes/parhaf_fix_rules.jsonl
regex-fixes/parrot_fix_rules.jsonl
regex-fixes/acronyms_fix_rules.jsonl
regex-fixes/manual_fix_rules.jsonl
plumed-rules/plumed_fix_rules.jsonl
regex-fixes/confusion_groups.jsonl
phrase-boosting/french_medical.txt
"

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
for file in $files; do
    cp "$data/$file" "$work/"
    # GitHub rejects 0-byte assets ("Bad Content-Length"); a lone newline
    # parses as an empty list (blank lines are skipped)
    name="$(basename "$file")"
    [ -s "$work/$name" ] || printf '\n' >"$work/$name"
done
cd "$work"

sha256() {
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$1" | cut -d' ' -f1
    else
        shasum -a 256 "$1" | cut -d' ' -f1
    fi
}

listing="$work/listing.tsv"
: >"$listing"
for file in $files; do
    name="$(basename "$file")"
    printf '%s\t%s\t%s\n' "$name" "$(sha256 "$name")" "$(wc -c <"$name" | tr -d ' ')" >>"$listing"
done

python - "$version" "$model" "$app_version" "$notes" "$listing" >manifest.json <<'EOF'
import json, sys
version, model, app_version, notes, listing = sys.argv[1:6]
files = []
with open(listing, encoding="utf-8") as f:
    for line in f:
        name, sha256, size = line.rstrip("\n").split("\t")
        files.append({"name": name, "sha256": sha256, "size": int(size)})
print(json.dumps({
    "version": version,
    "model": model,
    "min_app_version": app_version,
    "notes": notes,
    "files": files,
}, ensure_ascii=False, indent=2))
EOF
rm "$listing"

echo "Manifest:"
cat manifest.json

minisign -S -s "${PLUMED_RULES_KEY:-$HOME/.minisign/plumed-rules.key}" -m manifest.json
if [ -n "${PLUMED_RULES_PUBKEY:-}" ]; then
    minisign -V -P "$PLUMED_RULES_PUBKEY" -m manifest.json
fi

if ! gh release view "$tag" -R "$repo" >/dev/null 2>&1; then
    gh release create "$tag" -R "$repo" --latest=false --prerelease \
        --title "Medical data (not an app release)" \
        --notes "Signed medical data updates fetched by Plumed (see src-tauri/medical-data/SOURCE.md). Not an app release."
fi

# Data files first, the manifest and its signature last: an app checking in
# between sees the old manifest, or a manifest whose files do not match yet
# (refused, nothing installed).
data_files=""
for file in $files; do
    data_files="$data_files $(basename "$file")"
done
# shellcheck disable=SC2086
gh release upload "$tag" $data_files --clobber -R "$repo"
gh release upload "$tag" manifest.json.minisig manifest.json --clobber -R "$repo"

echo "Published medical data $version for $model (min app $app_version)."
