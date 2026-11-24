#!/bin/sh
# Builds plumed.fr in a Node container and publishes it to the folder Caddy
# serves. Made for the VPS, which keeps a sparse clone of the repo (website/,
# the root files, since the privacy page imports ../PRIVACY_POLICY.md, and
# src-tauri/icons and docs/screenshots, which the symlinks in website/ point to):
# deploy.sh pulls that clone and runs this script, and after editing the
# clone on the VPS directly, running `sh website/build.sh` there publishes it.
#
# Needs docker (through sudo) and rsync. The served folder is only replaced
# once the build succeeded. Settings, through the environment:
#   PLUMED_WEBSITE_OUT    folder Caddy serves (default
#                         ~/docker/caddy/caddy_folder/plumed_website)
#   PLUMED_WEBSITE_CACHE  pnpm and corepack cache (default: pnpm-cache next to
#                         the clone)
set -eu

WEBSITE_DIR=$(cd "$(dirname "$0")" && pwd)
REPO_DIR=$(dirname "$WEBSITE_DIR")
OUT_DIR=${PLUMED_WEBSITE_OUT:-$HOME/docker/caddy/caddy_folder/plumed_website}
CACHE_DIR=${PLUMED_WEBSITE_CACHE:-$(dirname "$REPO_DIR")/pnpm-cache}
NODE_IMAGE=node:22-slim
PNPM=pnpm@10.13.1

# Created by this user, not by docker as root
mkdir -p "$CACHE_DIR" "$OUT_DIR"

echo "Building the website in $NODE_IMAGE"
sudo docker run --rm \
    --user "$(id -u):$(id -g)" \
    -e HOME=/tmp \
    -e COREPACK_HOME=/cache/corepack \
    -e COREPACK_ENABLE_DOWNLOAD_PROMPT=0 \
    -e ASTRO_TELEMETRY_DISABLED=1 \
    -v "$REPO_DIR:/repo" \
    -v "$CACHE_DIR:/cache" \
    -w /repo/website \
    "$NODE_IMAGE" \
    sh -c "corepack $PNPM install --frozen-lockfile --ignore-workspace --store-dir /cache/store && corepack $PNPM build"

rsync -a --delete "$WEBSITE_DIR/dist/" "$OUT_DIR/"
echo "Published to $OUT_DIR"
