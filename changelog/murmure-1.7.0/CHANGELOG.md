# Murmure 1.7.0 (upstream) (2026-02-12)

**✨ New Features**

- **(Major) Select Command** — Select text and modify it with a custom command (e.g. fix grammar, translate to English, etc.).
- **(Major) LLM Prompts** — Support for multiple saved prompts with syntax highlighting for the prompt editor, anchor tags enforcement, and a new "Cursor Developer" preset.
- **Input Microphone Selection** — Allow selecting the input microphone in Settings. Thanks to @litel-fr.
- **Dictionary Import/Export** — Import and export words from the dictionary, including medical presets. Thanks to @icristescu ([#72](https://github.com/Kieirra/murmure/pull/72)).
- **Text Insertion Mode** — New setting to choose between standard, terminal-compatible, or direct typing insertion modes ([#119](https://github.com/Kieirra/murmure/issues/119)).
- **Log Verbosity** — Configurable log verbosity levels (trace, debug, info, warn, error) and a button to open the logs folder.
  **🐛 Bug Fixes**
- Fixed full screen issue on Select Model page on macOS ([#82](https://github.com/Kieirra/murmure/issues/82)).
- Transcription is no longer part of the logs by default ([#88](https://github.com/Kieirra/murmure/issues/88)).
- Temporary audio is now saved in the tmp folder instead of app_dir in case of crash ([#88](https://github.com/Kieirra/murmure/issues/88)).
- Fixed invalid word format error message in dictionary.
- Allow digit conversion threshold to be 0.
- Updated dependencies to fix security vulnerabilities.
  **♻️ Refactor**
- Fixed main SonarQube issues.
- Streamlined shortcuts logic and optimized event lookup.
  **🔧 CI**
- Added Security Scanning with SonarQube.
- Improved static analysis for contributions (test, compilation, linting, etc.).
- NSIS exe installer now runs without administration privileges.
