#!/usr/bin/env bash
# Plumed fork: fetch the UltiMed v2 Ultra int8 speech model into
# resources/parakeet-tdt-0.6b-v3-int8/. The app does not bundle it any more: it
# downloads its model on first launch (src-tauri/src/model/). Development builds
# (`pnpm tauri dev`, debug `cargo test`) fall back to this folder, and
# engine::beam's ignored test reads it, so a dev checkout works offline.
#
# Model: https://huggingface.co/Olicorne/parakeet-tdt-0.6b-v3-ultra-UltiMed-v2-onnx
# (UltiMed v2, French medical fine-tune of Parakeet TDT 0.6B v3).
#
# Usage:
#   scripts/fetch-ultimed-model.sh                        # download from Hugging Face
#   scripts/fetch-ultimed-model.sh path/to/clone          # copy from a local clone of the model repo
#   scripts/fetch-ultimed-model.sh --models-dir DIR       # also install it as the app's
#                                                         # downloaded model (CI smoke test)
#
# Every file is checked against the SHA-256 below. REVISION and the hashes
# repeat the ultra-int8 entry of src-tauri/src/model/catalog.rs: update both
# together. --models-dir writes the folder name, model.json and active.json the
# way src-tauri/src/model/store.rs does.
#
# Set up with Claude Code.
set -euo pipefail

REPO="Olicorne/parakeet-tdt-0.6b-v3-ultra-UltiMed-v2-onnx"
REVISION="675b4b8d7ee9f3ee15a273e3037f76ddc3e730bb"
DEST="resources/parakeet-tdt-0.6b-v3-int8"

# "<path in the model repo> <sha256>", installed flat into DEST
FILES=(
    "int8/encoder-model.int8.onnx 837c1689edd757b69f52b562b50667ab53f8d2693d58645108702c6621ca3fbd"
    "int8/decoder_joint-model.int8.onnx e1070b64c54a42c14172a6c8ee7e61e56dc92fdb7c4691da46b3944aa844de23"
    "nemo128.onnx a9fde1486ebfcc08f328d75ad4610c67835fea58c73ba57e3209a6f6cf019e9f"
    "vocab.txt d58544679ea4bc6ac563d1f545eb7d474bd6cfa467f0a6e2c1dc1c7d37e3c35d"
    "config.json 666903c76b9798caf2c210afd4f6cd60b08a8dbf9800ec8d7a3bc0d2148ac466"
)

sha256() {
    if command -v sha256sum > /dev/null; then
        sha256sum "$1" | cut -d' ' -f1
    else
        shasum -a 256 "$1" | cut -d' ' -f1 # macOS
    fi
}

MODELS_DIR=""
LOCAL_CLONE=""
case "${1:-}" in
    --models-dir) MODELS_DIR="${2:?--models-dir needs a folder}" ;;
    *) LOCAL_CLONE="${1:-}" ;;
esac
mkdir -p "$DEST"

for entry in "${FILES[@]}"; do
    path="${entry% *}"
    expected="${entry##* }"
    out="$DEST/$(basename "$path")"
    if [ -n "$LOCAL_CLONE" ]; then
        echo "Copying $LOCAL_CLONE/$path"
        cp "$LOCAL_CLONE/$path" "$out"
    else
        url="https://huggingface.co/$REPO/resolve/$REVISION/$path"
        echo "Downloading $url"
        curl --proto '=https' --fail --retry 3 -L -o "$out" "$url"
    fi
    actual="$(sha256 "$out")"
    if [ "$actual" != "$expected" ]; then
        echo "::error::SHA-256 mismatch for $path: expected $expected, got $actual" >&2
        exit 1
    fi
done

ls -al "$DEST"

if [ -n "$MODELS_DIR" ]; then
    installed="$MODELS_DIR/ultra-int8-${REVISION:0:12}"
    mkdir -p "$installed"
    cp "$DEST"/* "$installed/"
    printf '{"variant":"ultra-int8","revision":"%s","source":"hugging_face"}' "$REVISION" > "$installed/model.json"
    printf '{"variant":"ultra-int8"}' > "$MODELS_DIR/active.json"
    echo "Installed as the app's model in $installed"
fi
