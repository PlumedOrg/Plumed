"""Plumed fork (written with Claude Code): build the updater's latest.json.

The installed app downloads latest.json from the latest GitHub release to learn
whether a newer version exists, where its installer is, and the minisign
signature that proves it was built with our private key (checked against
`plugins.updater.pubkey` in src-tauri/tauri.conf.json).

Usage, after downloading the release workflow's artifacts into one folder
(subfolders are fine):

    python scripts/make-latest-json.py ARTIFACTS_DIR [--tag 0.1.0] [--notes "..."]
        [--allow-missing] [--add-to OLD_LATEST_JSON]

It writes ARTIFACTS_DIR/latest.json; upload it to the GitHub release next to
the installers. The version comes from package.json, the tag defaults to it,
and the repository comes from the updater endpoint in tauri.conf.json.

Every platform in REQUIRED must be present (a build still running or failed
would otherwise ship a release without it); --allow-missing publishes anyway.
A platform missing from latest.json means "pending": the app then reports no
update until it is added. --add-to adds the platforms found in ARTIFACTS_DIR
to an already published latest.json of the same version (a late build).
"""

import argparse
import json
import sys
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

# Updater platform key -> installer file name produced by the build workflows.
# The keys without an installer suffix are the fallbacks Tauri uses when it
# does not know how the app was installed; on Windows we prefer NSIS, our main
# installer (see AGENTS.FORK.md, Versioning).
PLATFORMS = {
    "linux-x86_64": "Plumed_amd64.AppImage",
    "linux-x86_64-appimage": "Plumed_amd64.AppImage",
    "linux-x86_64-deb": "Plumed_amd64.deb",
    "linux-x86_64-rpm": "Plumed_amd64.rpm",
    "windows-x86_64": "Plumed_x64-setup.exe",
    "windows-x86_64-nsis": "Plumed_x64-setup.exe",
    "windows-x86_64-wix": "Plumed_x64.msi",
    "darwin-aarch64": "Plumed_aarch64_darwin.app.tar.gz",
    "darwin-x86_64": "Plumed_x86_64_darwin.app.tar.gz",
}


# One fallback key per build workflow: if it is missing, that build is missing.
# The suffixed keys are optional (betas have no MSI, see AGENTS.FORK.md).
REQUIRED = ["linux-x86_64", "windows-x86_64", "darwin-aarch64", "darwin-x86_64"]


def release_download_base(tag: str) -> str:
    """Turn the updater endpoint (.../releases/latest/download/latest.json) into .../releases/download/<tag>."""
    conf = json.loads((ROOT / "src-tauri" / "tauri.conf.json").read_text())
    endpoint = conf["plugins"]["updater"]["endpoints"][0]
    repo, sep, _ = endpoint.partition("/releases/")
    if sep == "":
        sys.exit(f"Unexpected updater endpoint (no /releases/): {endpoint}")
    return f"{repo}/releases/download/{tag}"


def find_one(artifacts: Path, name: str) -> Path | None:
    matches = sorted(artifacts.rglob(name))
    if len(matches) > 1:
        sys.exit(f"Several files named {name} in {artifacts}: {matches}")
    return matches[0] if len(matches) == 1 else None


def found_platforms(artifacts: Path, base: str) -> dict:
    """Updater entries for the installers (with their .sig) present in artifacts."""
    platforms = {}
    for key, name in PLATFORMS.items():
        installer = find_one(artifacts, name)
        signature = find_one(artifacts, name + ".sig")
        if installer is not None and signature is not None:
            platforms[key] = {"signature": signature.read_text().strip(), "url": f"{base}/{name}"}
    return platforms


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("artifacts", type=Path, help="folder holding the installers and their .sig files")
    parser.add_argument("--tag", help="release tag (default: the version in package.json)")
    parser.add_argument("--notes", default="", help="release notes shown in the update dialog")
    parser.add_argument("--allow-missing", action="store_true", help="publish even if a REQUIRED platform is missing")
    parser.add_argument("--add-to", type=Path, help="published latest.json to add the found platforms to")
    args = parser.parse_args()

    version = json.loads((ROOT / "package.json").read_text())["version"]
    base = release_download_base(args.tag or version)
    platforms = found_platforms(args.artifacts, base)

    if args.add_to is not None:
        latest = json.loads(args.add_to.read_text())
        if latest["version"] != version:
            sys.exit(f"{args.add_to} is version {latest['version']}, package.json is {version}")
        added = sorted(set(platforms) - set(latest["platforms"]))
        replaced = sorted(set(platforms) & set(latest["platforms"]))
        print(f"adding {added}, replacing {replaced}")
        latest["platforms"] = {**latest["platforms"], **platforms}
    else:
        latest = {
            "version": version,
            "notes": args.notes,
            "pub_date": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
            "platforms": platforms,
        }

    if len(latest["platforms"]) == 0:
        sys.exit(f"No signed installer found in {args.artifacts}")
    for key in PLATFORMS:
        if key not in latest["platforms"]:
            print(f"warning: {key} missing ({PLATFORMS[key]} or its .sig not found)", file=sys.stderr)
    missing = [key for key in REQUIRED if key not in latest["platforms"]]
    if len(missing) > 0 and not args.allow_missing:
        sys.exit(f"Missing required platforms {missing}: wait for their build, or pass --allow-missing")

    out = args.artifacts / "latest.json"
    out.write_text(json.dumps(latest, indent=2) + "\n")
    print(f"wrote {out} ({len(latest['platforms'])} platforms, version {version}, pending: {missing})")


if __name__ == "__main__":
    main()
