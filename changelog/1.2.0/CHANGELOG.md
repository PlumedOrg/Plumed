# Plumed 1.2.0 (2026-11-24)

1.1.0 was never published: its changes are included here.

- **Documentation at [plumed.fr](https://plumed.fr)**: the help links in the app (dictionary number warning, Linux shortcut commands) lead to it again.
- **Phrase boosting tuning** (Paramètres > Corrections > Vocabulaire médical, at the bottom): the boost weight and margin (min-p) can be set, empty fields keep the bundled list's values (weight 0.5, min-p 0.01), a button restores them. Both are part of the settings export/import.
- **Local transcription API** (off by default, Paramètres > Avancé): an OpenAI-compatible `POST http://127.0.0.1:4800/v1/audio/transcriptions` endpoint that transcribes 16-bit PCM WAV files exactly like a dictation (dictionary, formatting rules, grammar, text templates), without writing to the history or the local stats.
  - Listens on 127.0.0.1 only, requires a random API key (shown, copyable and regenerable in the settings), refuses requests carrying an `Origin` header or a foreign `Host`.
  - Headless mode: `plumed serve [--port N]`, which refuses to start while the setting is off.
  - Port and key are never exported in `.murmure` files.
  - Usage statistics (if accepted): API transcriptions are counted apart (`api_*` aggregates) and a `local_api` yes/no flag says whether the API is on.
- **PhoneMic** (protocol 5.0.0, needs a relay at the same version):
  - Pairing by QR secret: clicking **Connecter** is enough, no more code to compare.
  - End-to-end acknowledgements: audio lost on the way is sent again automatically.
  - The phone resumes its session for 12 h after a reload.
  - Plumed checks once per session that the relay serves the genuine phone page, and a newer relay asks to update Plumed instead of failing.
  - Recordings waiting more than 30 s are dropped; a transcription ready more than 20 s after the end of the recording is kept in the history but not pasted.
- **Medical data updates**: optional, collapsed review of what an update changes.
- **Settings import**: lists the added, changed and turned-off entries of imported medical lists before applying them.
- **Usage statistics**: the opt-in shows the matching entry of the registre des traitements; the privacy policy now publishes the full registre and your rights.
