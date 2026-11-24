# Murmure 1.11.3 (upstream) (2026-08-31)

**✨ New Features**
- **Dictionary** — Word counter with a green/yellow/red indicator ([#386](https://github.com/Kieirra/murmure/issues/386)).
- **Dictionary** — Support the full Parakeet vocabulary and accept two-word entries, so expressions with a space or a hyphen can be added ([#386](https://github.com/Kieirra/murmure/issues/386)).
- **Dictionary** — Export as .txt, and accept .txt on import alongside the legacy .csv ([#386](https://github.com/Kieirra/murmure/issues/386)).
- **LLM** — New Transform shortcut per mode (`Ctrl+Alt+Shift+1` to `Ctrl+Alt+Shift+4`) that applies the mode's saved prompt to the selected text, with no dictation.
- **Shortcuts** — Support Pause and ScrollLock as shortcut keys on Windows and Linux ([#387](https://github.com/Kieirra/murmure/issues/387)).
- **Shortcuts** — Capture keys from the native backend so any detectable key can be recorded, fixing F13 shown as Unidentified and layout-dependent letter labels on X11 ([#388](https://github.com/Kieirra/murmure/issues/388)).
- **Shortcuts** — Allow binding the fn / globe key on macOS (thanks @gregoryrivage) ([#399](https://github.com/Kieirra/murmure/pull/399)).
- **Audio** — Lower the output volume while recording ([#364](https://github.com/Kieirra/murmure/issues/364)).
- **Audio** — Louder start and stop sounds, with a volume slider ([#426](https://github.com/Kieirra/murmure/issues/426)).
- **API** — Cancel the running transcription when the client closes the connection, and free the queue as soon as the abandoned work has stopped ([#411](https://github.com/Kieirra/murmure/discussions/411)).
- **Packaging** — New pacman package for Arch-based distros ([#358](https://github.com/Kieirra/murmure/issues/358)).
- **CLI** — New --hidden flag to launch Murmure without showing the window, combined with control commands such as --transcription or --llm-mode.
**🐛 Bug Fixes**
- **Audio** — Pad each chunk with silence before inference, fixing chunks the model could not decode which silently truncated the transcription.
- **Audio** — Adapt the voice detection to microphone gain and background noise for wake word, chunking, and silence auto-stop ([#381](https://github.com/Kieirra/murmure/discussions/381)).
- **Audio** — List Bluetooth microphones on Linux, PipeWire only sets the property the enumeration required on ALSA sources ([#421](https://github.com/Kieirra/murmure/issues/421)).
- **Audio** — Keep the output stream alive while recording, past one minute of dictation it closed on the idle timeout and the stop sound played on a device that had gone back to sleep ([#419](https://github.com/Kieirra/murmure/discussions/419#discussioncomment-18107863)).
- **Insert** — Type accented characters natively in Direct mode on Wayland instead of folding them to ASCII ([#384](https://github.com/Kieirra/murmure/issues/384)).
- **Insert** — Keep a copied image in the clipboard when dictating, instead of replacing it with empty text.
- **LLM** — Wait for the modifiers to be released before copying the selection, holding the shortcut turned the synthetic Ctrl+C into Ctrl+Alt+Shift+C so Transform reported an empty selection ([#419](https://github.com/Kieirra/murmure/discussions/419#discussioncomment-18114383)).
- **LLM** — Allow typing a custom model name and stop the connection test from locking the Remote provider, so OpenAI compatible servers without a /models endpoint can be used ([#397](https://github.com/Kieirra/murmure/issues/397)).
- **LLM** — Stop sending the temperature parameter to remote servers, OpenAI GPT-5 models only accept the default value and returned 400 Bad Request ([#397](https://github.com/Kieirra/murmure/issues/397)).
- **Shortcuts** — Wait for the modifiers to be released before pasting the last transcript, holding the binding merged it with the synthetic Ctrl+V so nothing was pasted (thanks @flomillot) ([#435](https://github.com/Kieirra/murmure/pull/435)).
- **Wake word** — Stop the microphone icon from flickering every 10 seconds during silence (thanks @nicob3y) ([#422](https://github.com/Kieirra/murmure/issues/422)).
- **Wake word** — Stop restarting the listener in a tight loop when the microphone is unavailable, which was flooding the log file ([#381](https://github.com/Kieirra/murmure/discussions/381)).
- **API** — Route /api/transcribe through the same chunked path as the keyboard and the CLI, so audio of any length works and the only limit left is the request size ([#401](https://github.com/Kieirra/murmure/issues/401)).
- **Logs** — Reset the log file above 1 MB while the app is running, instead of only checking at startup, and add an Off level to disable logging completely ([#417](https://github.com/Kieirra/murmure/issues/417)).
- **Packaging** — Set the Linux desktop file category so Murmure appears under Utility instead of Other ([#385](https://github.com/Kieirra/murmure/issues/385)).
- **Security** — Fix vulnerabilities flagged by Dependabot.
**🙏 Thanks**
- Beta testers: @tomsquest, @laurentkempe
- Contributors: @flomillot, @nicob3y, @gregoryrivage
- Donors: Nicholas, Nutwing, Ubertartine, Chase, Gizmo38, Aparrot, scwall, Prometee
