# Plumed 1.1.0 (2026-11-24)

- **PhoneMic** (protocol 5.0.0, needs a relay at the same version):
  - Pairing by QR secret: clicking **Connecter** is enough, no more code to compare.
  - End-to-end acknowledgements: audio lost on the way is sent again automatically.
  - The phone resumes its session for 12 h after a reload.
  - Plumed checks once per session that the relay serves the genuine phone page, and a newer relay asks to update Plumed instead of failing.
  - Recordings waiting more than 30 s are dropped; a transcription ready more than 20 s after the end of the recording is kept in the history but not pasted.
- **Medical data updates**: optional, collapsed review of what an update changes.
- **Settings import**: lists the added, changed and turned-off entries of imported medical lists before applying them.
- **Usage statistics**: the opt-in shows the matching entry of the registre des traitements; the privacy policy now publishes the full registre and your rights.
