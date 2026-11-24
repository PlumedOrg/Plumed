# Murmure 1.9.0 (upstream) (2026-05-28)

**✨ New Features**

- **(Major) Smart Speech Mic** — Use your phone as a wireless microphone by scanning a QR code in Murmure, no app install needed on the phone.
- **(Major) Native Wayland support** on Linux via CLI (thanks @BorisLord).
- **(Major) Overlay overhaul** — Configurable overlay size, real-time streaming preview with configurable text width, font size and max lines, and a color-coded visualizer per recording mode (standard, LLM, command).
- **Documentation** — Official Murmure documentation site (configuration, limitations, tips). https://kieirra.github.io/murmure/
- **Tray** — Recording indicator on the tray icon during transcription (visible even when the overlay is hidden); copy the last transcript directly from the tray menu.
- **Shortcuts** — Shortcut to toggle Voice Mode on/off ([#279](https://github.com/Kieirra/murmure/issues/279)); Ctrl+Shift+1-4 now directly trigger LLM transcription with the matching prompt (no separate trigger shortcut); hide the LLM Connect shortcut when the feature is disabled.
- **Layout** — New Extensions section in the menu; global UX/UI optimization to simplify the interface.
- **Rules** — Custom names for personal formatting rules.
- **Dictionary** — Warn when entering digits in a dictionary entry and link to the formatting rules.
- **Import/Export** — Include Voice Mode and Smart Mic categories.
- **Linux** — RPM bundle for Fedora-based distributions (thanks @BorisLord).
  **🐛 Bug Fixes**
- **Overlay** — Error messages no longer disappear too quickly and block subsequent recordings; fix streaming auto-scroll.
- **Overlay (Linux)** — Recording overlay is no longer focusable, preventing focus theft (thanks @flomillot) ([#344](https://github.com/Kieirra/murmure/pull/344)).
- **macOS** — Fix auto-update (wrong artifact format) ([#301](https://github.com/Kieirra/murmure/issues/301)); Cancel shortcut (Escape) no longer blocks other apps when not recording ([#302](https://github.com/Kieirra/murmure/issues/302)); tray icon adapts to light/dark mode using a template image (thanks @fwehrling); left click on the tray icon opens the menu instead of restoring the window.
- **Voice Mode** — Wake word now detected when spoken mid-sentence without a pause.
- **Shortcuts** — Some AZERTY keys (e.g. ²) could not be bound; clarify LLM Connect vs Command shortcut descriptions.
- **Linux / Wayland** — Disable Smart Mic and push-to-talk on Wayland; change the default Cancel shortcut on Wayland.
- **Sidebar** — Disable mobile mode to prevent it disappearing on high-scaling displays; use a bundled asset for the logo to avoid subroute 404 (thanks @BorisLord).
- **Formatting Rules** — Preserve user formatting rules when upgrading from pre-1.8.0.
- **Audio** — First start/stop sound after a cold start was sometimes silent (device not warmed up).
- **Visualizer** — More sensitive, works well with low-gain microphones.
- **Logs** — Log time now displayed in the correct timezone.
  **⚡ Performance**
- Shrink Rust binary by 32% (66 MB → 45 MB) and trim unused dependencies (thanks @BorisLord) ([#310](https://github.com/Kieirra/murmure/pull/310)).
- Skip Ollama warmup when the model is already loaded (thanks @BorisLord) ([#324](https://github.com/Kieirra/murmure/pull/324)).
  **🙏 Thanks**
- Beta testers ([discussion #346](https://github.com/Kieirra/murmure/discussions/346)): @laurentkempe, @Zabalero, @thiswillbeyourgithub, @anouar-coscientia, @tomsquest
- Contributors: @BorisLord, @fwehrling, @flomillot, @dadoonet
- Donors: @scwall, @stefounet, @Morpheus9057, @PaulMLH, @aparrot89, gizmo38, chase-14, floflow-3, migraine, ubertartine, b0g-3, frédéric-1477, madinia-1, sylvain-1123
