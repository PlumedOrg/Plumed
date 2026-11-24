# Plumed fork instructions

Rules and processes from the Plumed maintainer. Architecture notes live in [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md). Set up with Claude Code.

## 1. What this fork is

Plumed (https://github.com/PlumedOrg/Plumed, frontend constants in `src/utils/plumed-links.ts`) is a fork of [Murmure](https://github.com/Kieirra/murmure) for **French medical dictation**. It ports the medical mode of ParakeetWeb ([live](http://parakeetweb.olicorne.org/?mode=med), [code](https://github.com/thiswillbeyourgithub/parakeet_web), usually checked out in `../parakeet_web`) into a desktop app that dictates into any application.

Why a fork: [Kieirra/murmure#498](https://github.com/Kieirra/murmure/issues/498) (upstream prefers huge rule lists, ~30,000 regexes, and medical post-processing to live in a fork). Stay as close to upstream as possible and upstream anything generic.

## 2. Mandatory at the start of every session: check upstream

**Last checked upstream commit:** `8ad2363` (docs: update roadmap)
**Date of check:** 2026-11-12

1. `git fetch upstream` (remote `upstream` = `https://github.com/Kieirra/murmure.git`; add it if missing).
2. `git log --oneline <last checked commit>..upstream/main`.
3. Summarize the commits to the user with a merge/skip recommendation each, flagging those touching files our feature branches modify.
4. After approval, merge (section 4). Default: merge everything. To skip a commit, merge anyway then `git revert` it, so history stays mergeable.
5. Update hash, subject and date above and commit (`docs(fork): update last checked upstream commit`). Update the date even when nothing is new.

## 3. Minimal-diff rules (keep upstream merges painless)

- Prefer **adding new files/modules** over editing upstream ones; hook them in with the smallest change (one import, one call, one config entry).
- Never reformat, reorder, rename or "clean up" upstream code we don't need to change.
- Upstream's `GUIDELINES.md` and `CONTRIBUTING.md` are folded into section 7 and deleted, and `AGENTS.md` is only a pointer to this file: keep that on merges (take upstream's useful changes into section 7). Never edit other upstream docs except `README.md` (fully ours). Fork docs go in fork-specific files.
- Upstream's mkdocs site (`docs/`, `mkdocs.yml`, `overrides/`, `docs.yml` workflow) is deleted (it documents Murmure); keep the deletion on merges. User docs will live on plumed.fr; medical features are documented in the README.
- Large data (regex lists, phrase lists) goes in its own files, never inlined in upstream sources.
- Generic improvements must be upstreamable as is: no medical-specific code mixed in.
- Exception to upstream's "no persistence" rule: only when the user opted in to anonymous usage statistics, aggregated counters (sums, buckets, error counts, never text or per-dictation records) and a random id are persisted in `telemetry.json` (app data folder). Opting out deletes it. Anything sent must stay on the allowlist tested in `src-tauri/src/telemetry/payload.rs`.

## 4. Branching model

Branches only with the user's approval. **Never open a pull request upstream**: the user does that by hand.

- `medmain`: default, released branch = upstream + fork meta files + every finished feature merged in.
- `feat/<name>`: **one feature per branch**, branched from the last synced upstream commit (not `medmain`), so `git diff upstream/main...feat/<name>` shows only that feature. Merge into `medmain` with `git merge --no-ff`. A feature depending on another branches from its parent feature branch (note it in the table).
- Syncing: `git merge upstream/main` into `medmain` (never rebase `medmain`, it is published). Unmerged feature branches: rebase onto `upstream/main` (or merge if shared).
- `git config rerere.enabled true` is set locally.
- Recurring conflicts: `README.md` and version fields (section 5): always keep ours.

### Feature tracking

Update when a branch is created, merged or proposed upstream. "fork only" = upstream PR not planned.

| Feature | Branch | Status |
| --- | --- | --- |
| UltiMed as default model (downloaded on first launch, see "Speech models"; `scripts/fetch-ultimed-model.sh` for dev) | `feat/ultimed-model` | merged, fork only |
| Regex engine with lookarounds | `feat/regex-engine` | not needed (the only lookbehind, title exclusions, is handled in code) |
| Compile regex once | `feat/regex-compile-once` | merged, not planned upstream (see #498) |
| Medical regex list + dictation rules (`dictation_rules.csv`, read by `medical/dictation_rules.rs`) seeded as default user rules | `feat/medical-regex` | merged, fork only |
| Hybrid phrase boosting (upstream boost for the dictionary, ParakeetWeb trie for phrase lists, larger bonus wins) | `feat/phrase-boosting` (from `medmain`) | merged |
| French medical phrase list (armed at model load, ~2 s trie build) | `feat/medical-phrase-list` (depends on `feat/phrase-boosting`) | merged |
| Branding (name, purple theme, credits, "100 % local" card, identifier, updater, winget; icon and extra toggles TODO) | `feat/branding` (from `medmain`) | merged, in progress |
| Context for short dictations (under 2 s or at most 3 words: re-transcribed behind last 10 s of previous audio + 2 s silence, keep words after the gap; RAM only, 5 min expiry; live sessions only; setting on by default; bench `short_context::tests::bench_short_context`) | `feat/branding` | merged, fork only |
| Remote microphone "PhoneMic" (internal ids `remote_mic_*`; relay and phone page in `PhoneMic_server/`, HTTPS long-poll, E2E crypto pinned by `protocol/test-vectors.json`, pairing code confirmed on desktop; off by default, never exported) | `feat/branding` | alpha (`remote_mic::e2e::relay_e2e` passes with `PLUMED_RELAY_E2E_URL`; real phone pending, Safari untested), fork only |
| TDT beam search for final transcriptions (MAES width 5; preview stays greedy) | `feat/branding` | merged, fork only |

Removed upstream features (deliberate exception to minimal-diff, for local-first security): local HTTP API, Command mode, Prompt mode (LLM Connect, Ollama, remote LLMs, keyring API key, SmartMic LLM/translation modes, LLM and command wake words), SmartMic (LAN HTTPS/WebSocket phone mic, `build:smartmic`, its settings, export category, telemetry flag; replaced by PhoneMic), Voice mode (wake word, page, shortcut, `--voice-mode`, settings, export category, telemetry flag, overlay mode flash), and the "Getting started" checklist (`onboarding` key). Startup prompts and Wayland help cards in `src/features/onboarding/` stay. Old settings and `.murmure` files with `smartmic_*`, `wake_word_*`, `silence_timeout_ms`, `voice_mode_toggle_shortcut`, `onboarding` keys or `smartmic`/`voice_mode` categories still load (keys ignored). Network or text-leaving features are deleted outright, not hidden. On merges, keep them deleted. Exception: a local transcription API was later re-added as a Plumed-owned module (`src-tauri/src/local_api/`, off by default, 127.0.0.1 only, bearer key, Origin and Host checks, new `local_api_*` setting keys); do not merge upstream's HTTP API code into it.

## 5. Versioning

Own semver, independent of upstream, from `0.1.0`; betas as `0.2.0-beta.1` (`beta.1` < `beta.2` < `0.2.0`). Every release must bump it (the updater compares with `latest.json`). It must be identical in `package.json`, `src-tauri/Cargo.toml` (+ `Cargo.lock`) and `src-tauri/tauri.conf.json`; keep ours on merge conflicts. Upstream's version is not tracked (section 2 records the last merged commit).

WiX MSI only accepts numeric pre-releases, so `build-windows.yml` builds NSIS only when the version contains `-`. The NSIS `-setup.exe` is the main Windows installer (no admin rights; `latest.json` and winget use it).

## 6. Releases

Identity: identifier `org.olicorne.plumed`, product `Plumed`, binary `plumed` (`mainBinaryName`), so it installs next to Murmure: Linux package `plumed`, `/usr/bin/plumed`, resources, ONNX Runtime and pacman marker in `/usr/lib/Plumed/` (rpath set in `build-linux.yml`), release files `Plumed_*`. Upstream crate names (`murmure`, `murmure_lib`) and the `.murmure` format are kept.

ONNX Runtime: `ort` pinned at `2.0.0-rc.15` (ONNX Runtime 1.31.0); sessions share one telemetry-off environment (`ort_environment` in `engine/engine.rs`).
- Linux CI (`build-linux.yml`): `libonnxruntime.so` from NuGet `Microsoft.ML.OnnxRuntime` 1.31.0 (SHA-256 pinned), since `github.com/microsoft/onnxruntime` returned 404.
- Windows and Apple Silicon: `ort`'s `download-binaries`.
- Intel Mac: compiles v1.31.0 from source, unpatched (NuGet has no `osx-x64`; old Microsoft builds have AVX-VNNI in AVX2 kernels). Its VNNI detector is calibrated on Microsoft's 1.23.2 NuGet build (SHA-256 pinned). **TEMPORARY:** source fetched from third-party fork `mst-mkt/onnxruntime` pinned to `402979ac1bef89dcd55add41eaa7277c407599eb` (the commit in the 1.31.0 nuspec). TODO: remove the fork from the mirror list in `build-macos-intel.yml` when the official repo is back; never bump through it (take commits from Microsoft's nuspec).
- To bump: version + checksum in `build-linux.yml`, version + commit in `build-macos-intel.yml`.

Auto-updater (`src-tauri/src/app_update.rs`; key and stable endpoint in `plugins.updater` of `tauri.conf.json`): Rust does check and install via `updater_builder().endpoints(...)`; automatic checks (app and medical rules) run in `daily_checks.rs`: after the model loaded, then daily (hourly wall-clock poll), GUI only (`lib.rs` returns before it for `serve` / `transcribe`, whose hidden window still loads the frontend, so the frontend must never check on mount). Frontend `update-checker.tsx` reads `get_app_update_offer` + the `app-update-available` event, checks itself (`check_app_update`) only on a user channel change, and installs with `install_app_update`. Setting `app_update_channel` (Settings > Online, also `set_app_update_channel`; never exported):
- `stable` (default): `/releases/latest/download/latest.json` (skips pre-releases).
- `beta`: GitHub API, 10 newest releases; newest non-draft one with a `latest.json` asset wins.
- `disabled`: no automatic check; sidebar button checks stable on demand (otherwise the button only shows when an update exists).

Never downgrades (beta to stable waits for the next greater version). On any GitHub failure (network, timeout, HTTP error like a proxy 403) the check, the installer URL and failed installer downloads go through the relay (`models.plumed.fr`, `relay_base()` in `model/download.rs`), whose `/releases/...` routes mirror GitHub minus `/PlumedOrg/Plumed` (`PhoneMic_server/protocol/src/releases.rs`, routes in `PhoneMic_server/README.md`). Safe because only installers with a valid minisign signature against `pubkey` install. `latest.json` itself is deliberately unsigned: whoever controls it (GitHub, the relay, a proxy) can at worst hide an update, since a lower or equal version is never installed.

Updater key one-time setup:
1. `pnpm tauri signer generate -w ~/.tauri/murmuremed.key` (password). Keep the private key out of the repo and backed up: losing it means installed copies can never update.
2. Paste `murmuremed.key.pub` content as `pubkey`.
3. GitHub Actions secrets: `TAURI_PRIVATE_KEY` = output of `base64 -w0 ~/.tauri/murmuremed.key` (workflows `base64 -d` it; raw content fails) and `TAURI_KEY_PASSWORD`.

Each release:
1. Bump the version (section 5).
2. Write the notes in `changelog/<version>/CHANGELOG.md`, starting with `# Plumed <version> (YYYY-MM-DD)`. The root `CHANGELOG.md` is only an index: never add notes there.
3. Run the `Release Plumed` workflow, create a GitHub release tagged with the version from its artifacts (betas like `0.2.0-beta.1` marked pre-release).
4. `python scripts/make-latest-json.py <artifacts folder> --notes "..."` and upload `latest.json` (installer URL + `.sig` per platform, format https://v2.tauri.app/plugin/updater/#static-json-file; workflows don't generate it). Every release, beta included, needs it.

Build durations for ETAs (1.2.0, November 2026, job start to end, waiting for a runner excluded): Linux 15 min, Windows 14 min, Apple Silicon 16 min, Intel macOS 21 to 29 min (it compiles ONNX Runtime; the rust cache makes the difference). The jobs run in parallel, so a full release takes about 30 min once every runner is acquired. The wait for a runner is not included: macOS Apple Silicon runners (`macos-15`) were often cancelled with "not acquired" and took hours and several reruns (`gh run rerun <id> --failed`, only once the run is completed).

Releases touching `PhoneMic_server/` (the relay has no backward compatibility on purpose, see `PhoneMic_server/protocol_details.md` section 0):
- Phone page only (`server/static/`): copy the previous page (from git) to `server/pages/<old PAGE_VERSION>/`, bump `PAGE_VERSION` in `protocol/src/page.rs`, update the checksum pin of its test. The relay serves every page under `/p/<version>/` and each Plumed links to and checks its own, so nobody is cut off: deploy the relay before publishing the app, any time before. Archived pages are kept forever (cheap, and protocol bumps empty them anyway).
- Protocol (`PROTOCOL_VERSION`): empty `server/pages/`, bump `PAGE_VERSION` too, regenerate the test vectors. Apps and relays on different protocols refuse each other, so build the release first without publishing, then publish it and deploy the relay back to back (the upstream author does both with one local script, since the deploy asks for an ssh passphrase).
- Anything else in `PhoneMic_server/`: deploy the relay whenever, as long as Plumed does not depend on the change.

Code signing: no Apple developer account ever, so macOS is unsigned, not notarized (users allow it in Privacy & Security or `xattr -dr com.apple.quarantine /Applications/Plumed.app`). Apple cert/DMG/notarization steps run only if `MACOS_CODESIGN_P12_B64` exists, else ad-hoc signing (`APPLE_SIGNING_IDENTITY=-`, required on Apple Silicon). Windows SignPath runs only if `SIGNPATH_API_TOKEN` exists, else unsigned (SmartScreen). The updater `.sig` is always made.

WinGet (`.github/workflows/publish-winget.yml`, id `Olicorne.Plumed`, NSIS only): runs on every published release but only updates an existing package in microsoft/winget-pkgs. Submit the first version by hand (`komac new` or `wingetcreate new`), then add secret `WINGET_TOKEN` (token allowed to open PRs on winget-pkgs).

## 7. Coding conventions

Inherited from upstream Murmure (its GUIDELINES.md and CONTRIBUTING.md, folded in here).

Principles: privacy first (no user data persisted beyond the last five transcriptions, except the opt-in statistics of section 3), no security compromise (no open CORS, no unsafe shortcut), clean and simple code (single responsibility, no duplication, no over-engineering), small focused commits and PRs (one concern each, no drive-by cleanups).

Frontend (React + TypeScript):
- Files and folders in kebab-case (`history-item.tsx`, `use-audio-recorder.ts`, `history.helpers.ts`); components in PascalCase, hooks `useCamelCase`, helpers camelCase.
- Feature-first: `src/features/<page>/<page>.tsx` entry, `<page>.helpers.ts`, `hooks/use-<hook>.ts`, `<sub-component>/<sub-component>.tsx`. Shared primitives in `src/components/`. No barrel files (`index.ts`): import the source file.
- Function components only (no `React.FC`, no classes); keep markup simple, logic in hooks or helpers.
- Styling: Tailwind and shadcn/ui only, icons from lucide-react, no other UI or CSS library, no component CSS files. Semantic HTML and ARIA labels on interactive elements.
- `interface` for object shapes and props (no `I` prefix), `type` for unions and derived types. `any` is forbidden; narrow `unknown` with guards.
- Explicit conditions: `if (items.length > 0)`, `if (value != null)`, never `if (items.length)`.

Backend (Rust):
- Rust API naming (snake_case, PascalCase types, UPPER_SNAKE_CASE constants).
- One feature = one directory: `mod.rs` only re-exports, entry file named after the directory (`audio/audio.rs`), helpers in `helpers.rs`, types in `types.rs`. `main.rs` minimal, setup in `lib.rs`.
- Fallible functions return `Result<T, E>`; `panic!` only for bugs. Prefer `match` over `if let`/`else if` chains on enums.
- `cargo fmt` and `cargo clippy` clean before committing; be sparing with new dependencies, run `cargo audit`.

Development setup: install the [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/), run `bash scripts/fetch-ultimed-model.sh` once (development builds use it while no model is downloaded), then `pnpm install` and `pnpm tauri dev`.

AI-assisted contributions are welcome, but the contributor answers for every line: understand it, challenge it, test it by hand. Untested, unreviewed AI output gets closed.

## 8. Other

- Backlog and tasks: organize them in `TODO.local.md` (gitignored), not in ad-hoc note files: add queued requests there, remove items once done.
- Docs stay in sync with code: every code change updates the related documentation in the same commit, so nothing goes stale. This matters most for the PhoneMic protocol (`PhoneMic_server/protocol_details.md`, `PhoneMic_server/README.md`, `PhoneMic_server/protocol/`): any change to the wire format, handshake, crypto or relay routes must update them.
- Language: Plumed targets French users, so user-facing documentation (README, user docs, in-app text, PhoneMic server docs) is written in French. Code, code comments and markdown aimed at developers or coding agents (AGENTS*.md, docs/, changelog) stay in English.
- Machine-specific notes (testable OSes): `AGENTS.local.md` (gitignored), read it if present.
- Architecture and feature notes: [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md), included below.

---

I opted to put in the default Claude Code context the readme, privacy policy and architecture file so you never get them out of sync.

@AGENTS.local.md

@docs/ARCHITECTURE.md

@PRIVACY_POLICY.md

@README.md
