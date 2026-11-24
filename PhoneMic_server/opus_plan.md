# PhoneMic: Opus instead of raw PCM (design note, not implemented)

Investigation written with Claude Code on 2026-11-24. Reference: ParakeetWeb's remote-mic v4 (commits 550ef36b, 14eee540, 701ca091), which sends Opus over WebCodecs with a PCM fallback.

## Why

The phone already downsamples to 16 kHz (`audio.js` `Downsampler`), so today's stream is 16,000 samples/s x 2 bytes = 32 kB/s (256 kbit/s). Opus at 24 kbit/s is 3 kB/s, about 10x less relay traffic. A 1-minute dictation goes from 1.9 MB to 180 kB.

## What differs from ParakeetWeb (and makes Plumed simpler)

- ParakeetWeb decodes with the browser's `AudioDecoder`, which may lack Opus and which outputs 48 kHz whatever is asked (hence commit 14eee540's decimation). Plumed decodes in Rust with libopus, which is always there and decodes at the rate it is asked for (8, 12, 16, 24 or 48 kHz). So the desktop never needs to advertise codecs and the 48 kHz trap does not exist.
- The only side that may lack Opus is the phone: no `AudioEncoder` (older iOS Safari), `isConfigSupported` false at the capture rate, or an encoder error in the middle of a recording.
- ParakeetWeb keeps a full Int16 copy of every Opus recording (up to 10 min) until the desktop's `codec-ok`, and re-sends it all as PCM on `codec-nack`. Plumed already has end-to-end ACKs (protocol 5.0.0), which give a much tighter bound: see "Fallback" below.

## Protocol 6.0.0 changes

- `0x01` START payload: `sample_rate(u32 BE)` then `codec(u8)`: `0` = PCM s16le (today), `1` = Opus. The phone chooses per recording.
- `0x02` AUDIO with codec Opus: 20 ms packets grouped like ParakeetWeb, payload = repeated `[u16 BE length][packet]`, 25 packets (500 ms) per frame, the same cadence as today's PCM batches. 20 ms at 16 kHz = 320 samples per packet, so the sample count of each frame is known exactly from its packet count.
- New `0x05` SWITCH (phone to desktop): `sample_offset(u64 BE)`, "the rest of this recording is PCM from this sample on". The desktop truncates its decoded buffer to `sample_offset` (or pads with silence if short, which should not happen) and appends the following AUDIO frames as PCM.
- New `0x15` CODEC_NACK (desktop to phone): `seq(u64 BE)`, the last packet decoded fine. The desktop refuses Opus for the rest of the pairing.
- ACK semantics, unchanged on the wire: for an Opus frame, the desktop decodes it inside `on_binary` before moving `seq_in`, so an ACKed Opus packet is a decoded one.
- Bump `PROTOCOL_VERSION` (mixed into `HKDF_SALT`), `PAGE_VERSION`, archive the 5.x page, regenerate `test-vectors.json`, update `protocol_details.md` (French) and the archived-pages rules.

## Fallback without re-recording, and with almost no re-sending

The phone keeps the PCM (Int16, 16 kHz) of every Opus frame still in the `Outbox`, i.e. not ACKed yet. Under normal conditions that is a few seconds of audio, never more than what `MAX_BYTES` already bounds. Cases:

1. Phone cannot encode Opus at all (no `AudioEncoder`, unsupported rate such as 11,025 Hz, `isConfigSupported` false): START says PCM, exactly today's path.
2. Encoder error mid-recording (phone side): the phone knows how many samples it has sent as Opus (packets x 320). It sends SWITCH(that count) and carries on in PCM from its live capture. Nothing is re-sent.
3. Decode error on the desktop (corrupted encoder output, libopus refusal): the desktop sends CODEC_NACK(last good seq) and drops every later Opus frame of this recording. The phone takes the PCM copies of the Opus frames after that seq (already in its Outbox, since they were not ACKed), sends SWITCH(samples decoded so far) followed by those samples as PCM, then continues in PCM. Only the audio the desktop could not decode is re-sent, typically under 1 s, and the user never repeats anything.
4. Phone reload mid-recording: unchanged, the next START abandons the unfinished recording (as in 5.0.0).

Example of case 3, with 16-packet ACKs: Opus frames seq 40 to 47 sent (8 x 500 ms), the desktop decoded 40 to 44 and fails on 45. It answers CODEC_NACK(44). The phone still holds frames 41 to 47 unACKed (last ACK was 40), drops 41 to 44 from its retained PCM (implicitly ACKed by the NACK), sends SWITCH(samples up to the end of 44) then the PCM of 45 to 47 (1.5 s, 48 kB), then live PCM.

Pre-skip: libopus output starts with the encoder's priming (312 samples at 48 kHz, 104 at 16 kHz). The desktop drops it at the start of each Opus recording, so the decoded sample count matches the phone's packet x 320 accounting used by SWITCH.

## Work items

- Desktop (`src-tauri`):
  - Opus decoder crate: `opus` (binds libopus through `audiopus_sys`, which can build the bundled libopus with cmake on all three CI targets) or a pure-Rust decoder if one is mature enough (to evaluate: avoids a C build on Windows). One decoder per recording, created at START.
  - `remote_mic/protocol.rs`: `recording` becomes `{ pcm, rate, codec, decoder }`; AUDIO decodes or appends; SWITCH; CODEC_NACK emission; refuse Opus after a NACK; the 20 min cap counts decoded samples.
  - Tests: an Opus recording round trip (encode with the same crate in the test), SWITCH truncation, NACK then PCM continuation, oversize or malformed packet groups (length prefix past the end) refused without panic.
- Phone page (`PhoneMic_server/server/static/`):
  - `opus.js` (port of ParakeetWeb's `remote-mic-opus.js`, minus the decoder side): probe `AudioEncoder` at 16 kHz once, encode at 24 kbit/s, 20 ms frames, group 25 packets, flush before END.
  - `app.js`: choose the codec at START, keep the Int16 copy alongside each Outbox item, handle encoder errors (SWITCH) and CODEC_NACK.
  - `outbox.js`: items carry their PCM copy and sample range; MAX_BYTES counts both.
- Relay: nothing (it only sees encrypted frames). Its per-frame size limits stay valid since frames shrink.
- Docs: `protocol_details.md`, `PhoneMic_server/README.md`, `PRIVACY_POLICY.md` (the relay sees smaller packets: same metadata, less volume), `docs/ARCHITECTURE.md`.

## To measure before shipping

- WER through the codec: round-trip the 400 benchmark clips through libopus at 16 kHz and 16, 24 and 32 kbit/s (`ffmpeg -c:a libopus -b:a 24k`), run `bench_transcribe`, compare with the PCM WER. Choose the lowest bitrate with no measurable loss (ParakeetWeb uses 48 kbit/s without having measured).
- Which iOS versions have `AudioEncoder` with Opus (case 1 above covers the rest), on a real iPhone.

Rough size: a few hundred lines on each side plus tests, comparable to the 5.0.0 ACK change.
