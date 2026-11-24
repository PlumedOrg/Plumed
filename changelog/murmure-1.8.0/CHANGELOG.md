# Murmure 1.8.0 (upstream) (2026-03-28)

**✨ New Features**
- **(Major) Voice Mode** — Activate Murmure by voice with customizable wake words, no keyboard needed. Includes auto-send enter, and wake word listener stays active during keyboard-triggered recording.
- **(Major) LLM Remote Servers** — Connect to remote OpenAI-compatible API servers for LLM Connect, with per-mode provider switching, model search, per-mode voice triggers, and onboarding flow.
- **(Major) Import/Export Settings** — Import and export configuration settings partially or fully. Add CLI import support (`murmure import <file>`) for sysadmin mass deployment.
- **Shortcuts** — Add support for mouse buttons, OEM keys (-, =, [, ], ;, ', etc.), <>, ², F13-F24. Add a shortcut in overlay to cancel recording.
- **Rules** — Add regex support for custom rules, improve rule labels, add "?" helper for replacement text, short text correction (auto-lowercase/remove punctuation), and rule reordering.
- **LLM** — Allow LLM mode reordering. Add "Voice Dictation" preset for interpreting spoken formatting commands as typographic characters.
- **Dictionary** — Add option to clear all dictionary entries.
- **About Page** — Redesigned with starry background and sponsor cards.
- **Autostart** — Start minimized to tray on autostart.
- **macOS** — Add configurable option to show/hide application in Dock.
- **Linux** — Add `.deb` package and `install.sh` one-liner script for Debian/Ubuntu.
- **Update** — Update `tauri-plugin-updater` to 2.10.0 for multi-format support (.deb, .exe, macOS Intel).
- **Sidebar** — Add "Release notes" link in sidebar footer.
- **Defaults** — Improve default settings for new users (transcription in RAM, trailing space).
**🐛 Bug Fixes**
- Fix overlay scrollbar on Windows, overlay stealing focus, overlay freeze on audio timeout.
- Fix double percentage display during download.
- Use dark mode colors consistently.
- Improve microphone list and keep manually selected device when disconnected. Fix crash when no microphone is available.
- Improve LLM accuracy with system prompts. Improve response speed of reasoning models by disabling thinking mode.
- Improve smart mode auto-spacing and word deduplication (parakeet fix).
- Fix selected text detection when clipboard already contains the same content.
- Fix ToggleToTalk accidental restart caused by X11 auto-repeat.
- Fix spurious modifier key release on macOS after paste and recording lock management.
- Fix wake word language-aware validation, history update, and EMA smoothing for VAD.
