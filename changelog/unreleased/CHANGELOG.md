# Plumed (unreleased)

- **Local transcription API** (off by default, Paramètres > Avancé): an OpenAI-compatible `POST http://127.0.0.1:4800/v1/audio/transcriptions` endpoint that transcribes 16-bit PCM WAV files exactly like a dictation (dictionary, formatting rules, grammar, text templates), without writing to the history or the local stats.
  - Listens on 127.0.0.1 only, requires a random API key (shown, copyable and regenerable in the settings), refuses requests carrying an `Origin` header or a foreign `Host`.
  - Headless mode: `plumed serve [--port N]`, which refuses to start while the setting is off.
  - Port and key are never exported in `.murmure` files.
  - Usage statistics (if accepted): API transcriptions are counted apart (`api_*` aggregates) and a `local_api` yes/no flag says whether the API is on.
