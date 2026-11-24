# Murmure 1.6.0 (upstream) (2025-12-15)

**✨ New Features**
- **(Major) LLM Connect** — Connect Murmure to a local LLM (Ollama) to enhance, correct, or reformat transcriptions through post‑processing. Includes presets for translation, medical dictation, development, and automatic reformulation.
- **(Major) Formatting Rules** — Automatic text transformations: add trailing spaces, insert spaces before `?!`, convert text-to-numbers, or apply custom find/replace rules.
- **Toggle-to-Talk Mode** — New alternative to push‑to‑talk: press once to start recording, press again to stop. Thanks to @nicolascrop.
- **Sound Feedback** — Audio cues when starting and stopping a recording.
**♻️ Refactor**
- Improved UI notifications.
- Backend refactor aligned with SRP principles.
- Dictionary configuration migration in preparation for import/export support. Thanks to @icristescu.
**🐛 Bug Fixes**
- Fixed overlay misalignment with fractional display scaling. Thanks to @flomillot (PR #73).
