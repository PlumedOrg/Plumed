# Plumed 1.4.0 (TODO)

- **Réunion page (experimental)**: transcribes a meeting of up to 4 hours and tells up to 4 speakers apart, from the microphone, PhoneMic or an audio file (WAV, MP3, M4A, FLAC, OGG), all on the computer. Lines read `- Paul: ...`, speakers can be renamed, timestamps are optional (from the start or from a clock time, with an offset), and the transcript can be copied. Stopping and starting again continues the same meeting. Voices are recognized, so a speaker who comes back after a long silence, or was first taken for two people, gets their letter back, earlier lines included. The speaker and voice models (about 134 MB) are downloaded on demand. Medical spelling fixes apply, no other correction. The computer does not fall asleep on its own while a meeting is transcribed, and dictation is refused meanwhile (the overlay says so).
- **Meeting backup (off by default)**: the audio, the transcript and the progress can each be saved as the meeting goes, in the temporary folder or in the app folder. After a crash, Plumed offers at startup to resume the meeting or delete the backup; with the audio saved from the start, the speakers keep their letters after the resume. "New meeting" deletes it.
- **Slow down the audio (experimental, off by default)**: slows your voice by 25% (pitch kept) before transcribing, which helps fast speakers (word error rate 5.2% to 4.7% when speaking 1.5x faster in our tests).
- **The first model download** now blocks the app in a progress dialog, so it cannot be used half installed.
- **The recording overlay** shows once the microphone is actually open.
- **F21 to F24** can be used in shortcuts.
- The app name is spelled right when dictated (« plume med », « plumaide »... become Plumed).
