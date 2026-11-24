# Murmure 2.0.3 (upstream) (2026-10-19)

**✨ New Features**

- **(Major) New interface**: every page was redesigned. Settings now comes before Extensions, the getting started checklist moved to the sidebar, and each extension page has its own color with its on/off switch next to the title.
- **(Major) Parakeet ultra model**: a fine-tune of Parakeet by Moondream, in the int8 export by @thiswillbeyourgithub. It makes fewer mistakes and uses about 380 MB less RAM.
- **(Major) Command Mode**: Command Mode is now a separate extension with its own model. With nothing selected, a command answers your question instead of repeating it.
- **Prompt Mode**: LLM Connect is renamed Prompt Mode. The server options are now named Ollama and OpenAI-compatible, and the privacy warning only shows for a server outside your computer or local network.
- **Local API**: no longer experimental, with its own extension page, new endpoints that apply a Prompt Mode prompt, and a **Try it** button to test each endpoint with a WAV file.
- **Overlay**: the result can stay on screen after a dictation, with a copy button ([#443](https://github.com/Kieirra/murmure/discussions/443)).
- **Settings**: new **Automatic insert** switch, separate from the insertion method ([#449](https://github.com/Kieirra/murmure/issues/449)).
- **CLI**: new `--quit` flag to close Murmure ([#429](https://github.com/Kieirra/murmure/issues/429)).
- **Startup**: the interface shows right away while the model loads in the background ([#500](https://github.com/Kieirra/murmure/issues/500)).
  **🐛 Bug Fixes**
- **Voice Mode**: fewer false wake word triggers, short words such as "ok" must now match exactly.
- **Transcription**: hesitation sound removal is optional and off by default, as it also removed real words such as the German "um" ([#445](https://github.com/Kieirra/murmure/issues/445)). Transcriptions no longer contain a stray `<unk>` token.
- **Windows**: dark title bar, scrollbars and native controls ([#500](https://github.com/Kieirra/murmure/issues/500)).
- **macOS**: clicking the Dock icon reopens the window, thanks @linkermaxx ([#439](https://github.com/Kieirra/murmure/pull/439)). The overlay shows over full-screen apps, and the Show in Dock setting explains its effect ([#437](https://github.com/Kieirra/murmure/issues/437)).
- **Linux**: the volume of audio started during a dictation, such as a new browser tab, is restored at the end.
- **Updater**: downloaded updates keep their real file extension ([#481](https://github.com/Kieirra/murmure/issues/481)).
- **i18n**: the tray menu is translated ([#451](https://github.com/Kieirra/murmure/discussions/451)).
- **Security**: the Smart Mic pairing token moved to the URL hash ([#472](https://github.com/Kieirra/murmure/pull/472)), the local API rejects requests from a foreign origin ([#475](https://github.com/Kieirra/murmure/pull/475)), recording files are owner-only ([#476](https://github.com/Kieirra/murmure/pull/476)), the .deb is checked before install ([#479](https://github.com/Kieirra/murmure/pull/479)), the Ollama URL is validated ([#477](https://github.com/Kieirra/murmure/pull/477)), and dependencies got their security updates.
  **🙏 Thanks**
- Beta testers: @laurentkempe, @Choupinounours, @ldmpub, @Martin511, @HachetAmaury, Anaïs
- Contributors: @xop01, @linkermaxx, @thiswillbeyourgithub
- Donors: Migraine, Ubertartine, Chase, Gizmo38, Aparrot, Prometee, dmxplx, DrROBINN
