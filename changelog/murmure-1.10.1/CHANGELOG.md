# Murmure 1.10.1 (upstream) (2026-07-07)

**✨ New Features**

- **Audio** — Higher-quality audio resampling for better transcription accuracy, especially on low-end microphones.
- **Audio** — New custom Parakeet model by @thiswillbeyourgithub, optimized with SmoothQuant for better accuracy and fewer unwanted switches to English.
- **Audio** — Silence-based chunking that splits recordings to remove the 5-minute limit and reduce English switches on long dictation ([#359](https://github.com/Kieirra/murmure/issues/359)); chunks are transcribed in the background while you keep speaking, so long dictations finish much faster.
- **CLI** — New `murmure` command to transcribe audio from the terminal.
- **Dictionary** — Redesigned UI, word boosting via Parakeet for better accuracy ([#338](https://github.com/Kieirra/murmure/issues/338)), and alphabetical sorting.
- **Overlay** — Close button to cancel an ongoing transcription; the active LLM prompt name is shown on recording start.
- **Home** — Redesigned home page.
- **Tray** — Monochrome tray icons (idle and recording) on Linux and macOS.
- **Insert** — None option for Text Insert Mode to disable auto-insertion ([#349](https://github.com/Kieirra/murmure/issues/349)).
- **Smart Mic** — Silence chunking and a 20-minute limit.
- **Shortcuts** — Delete key removes the selected shortcut.
- **Settings** — Debug option to keep the last five audio recordings, with a button to open the folder.
- **About** — Star-on-GitHub card.
  **🐛 Bug Fixes**
- **Audio** — Fix crackling, robotic recordings caused by dropped capture periods on some Linux setups; open the feedback sound stream lazily and release it when idle to stop keeping Bluetooth devices active ([#368](https://github.com/Kieirra/murmure/issues/368)).
- **Overlay (macOS)** — Click-through and cancel button now work; poll the cursor to toggle click-through reliably.
- **Shortcuts** — Prevent adding a duplicate shortcut; fix recording restarting after transcription when the shortcut uses only Ctrl and/or Shift ([#377](https://github.com/Kieirra/murmure/issues/377)).
- **Tray** — Tooltip shows the app name on hover.
- **Security** — Fix vulnerabilities flagged by Dependabot.
  **♻️ Refactor**
- **i18n** — Rename the Custom Dictionary label to Dictionary.
  **🙏 Thanks**
- Beta testers: @HachetAmaury, [Walden](https://github.com/Kieirra/murmure/discussions/381#discussioncomment-17523778), [CollectifWeb](https://github.com/Kieirra/murmure/issues/365#issuecomment-4897089011)
- Contributors: @thiswillbeyourgithub, @flomillot, @blouflab
- Donors: Ubertartine, Chase, Gizmo38, Aparrot, Raphael, Stephen Berard, B0g, AndraaX, Prometee, scwall, thomasdiluccio
