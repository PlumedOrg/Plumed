# Plumed remote microphone relay (alpha)

> **Alpha:** the remote microphone is a pre-release feature, expect bugs and breaking protocol changes between Plumed versions.

This directory holds everything that runs on the server side of the Plumed remote microphone: a small Rust (axum) HTTPS relay that also serves the phone page. It was written with Claude Code.

How it works:

1. Plumed (desktop) creates a room on the relay and shows a QR code: `https://turn.plumed.fr/#{room}.{phone secret}.{desktop public key}`. The room has one secret per side, and the QR code only carries the phone's: whoever photographs it cannot read what phones send to the desktop nor close the room.
2. The phone opens the page, generates its own key and posts a `hello` with its public key and the random name of its own inbox (so phones of the same room never take each other's frames). Both screens show the same pairing code (for example `524C-F8E8-D5A4-2D82`) and the desktop user confirms it. The confirmation (encrypted) names a new room opened for this phone alone, and the phone moves there. The QR code then changes to a new room too, so a photo of an old QR code opens nothing, and someone holding the current one can at most crowd its 8 phone inboxes to delay the next pairing, never disturb the bound phone.
3. From then on every frame is encrypted end to end (ECDH P-256, HKDF-SHA256, AES-256-GCM), with one key per direction bound to both public keys, so a packet can be neither reflected back to its sender nor replayed into another pairing. Refusals and "another phone took over" notices are encrypted too, so the relay cannot forge them. The relay only forwards opaque bytes between inboxes: it cannot hear anything, keeps everything in RAM, and logs no IP or room id. This holds against the relay operator and the network as long as the phone page itself is honest: the same server serves the page and its crypto code, so a compromised relay could serve a modified page. Use a relay you or your institution trust, or host your own.

Transport is plain HTTPS long-polling (no WebSocket, no WebRTC, no TURN server), so it works behind any proxy or firewall that lets HTTPS through.

## Layout

| Path | Content |
| --- | --- |
| `protocol/` | Rust crate shared with Plumed (`src-tauri`): envelope framing, roles, `seq \|\| op \|\| payload` plaintext, and `test-vectors.json` |
| `server/src/` | The relay (rooms, rate limiter, HTTP API, embedded phone page) |
| `server/static/` | The phone page (plain ES modules, no build step): French by default, English toggle |
| `server/static-tests/` | Node tests of the phone JS (wire, crypto against the shared vectors, VAD) |
| `Dockerfile`, `docker-compose.yml`, `env.example` | Deployment |

The phone's `wire.js` and `crypto.js` mirror the Rust code (`protocol/src/lib.rs`, `src-tauri/src/remote_mic/crypto.rs`). This duplication across languages is unavoidable, so both sides are pinned by `protocol/test-vectors.json`: change them together.

## Phone page features

- Manual mode: tap to start and stop, or hold the button for push-to-talk.
- Hands-free mode: the phone listens continuously and only sends speech. An energy VAD with an adaptive noise floor (20 ms frames) opens an utterance after 60 ms of speech, includes the 400 ms heard before it, closes it after 900 ms of silence, and drops it when it held less than 250 ms of speech. Silence never leaves the phone.
- Pause and resume, sending an audio file (20 minutes max).
- Screen wake lock while connected, so the phone does not fall asleep.
- Reconnects by itself after a network loss. The session keys live in memory only and the room secret is removed from the address bar once read: a page reload needs a new scan of the QR code.

## HTTP API

| Request | Answer |
| --- | --- |
| `POST /api/rooms` | `201 {"room","desktop","phone"}` (one secret per side), `403` (sent by a browser: only the desktop app creates rooms), `429` (per-IP limits) or `503` (server full or busy) |
| `POST /api/rooms/{room}/desktop` + phone secret | Appends an envelope to the desktop inbox: `204`, `400` (malformed), `429` (inbox full: 1 MiB, or 64 MiB for the whole relay) |
| `GET /api/rooms/{room}/desktop` + desktop secret | Long-poll (25 s): `200` with the queued envelopes, `204` if nothing came, `404` once the room is closed |
| `POST /api/rooms/{room}/phone/{tag}` + desktop secret | Appends an envelope to that phone's inbox (`tag`: 16 to 43 base64url characters chosen by the phone) |
| `GET /api/rooms/{room}/phone/{tag}` + phone secret | Long-poll of that phone's inbox. At most 8 phone inboxes per room: inboxes idle for a minute make room, otherwise `429` |
| `DELETE /api/rooms/{room}` + desktop secret | `204` |
| `GET /healthz` | `ok` |

Secrets go in the `x-room-secret` header. Unknown room and wrong secret (including the other side's) both answer `404`. Rooms are dropped after 10 minutes without polling or 24 hours of age.

Any request can also get `429` when its client already has 4 requests in flight per room it may open (400 with the default `PLUMED_RELAY_ROOMS_PER_IP`), or `503` when the relay is busy (2048 requests in flight, or 32 MiB of bodies being read). A body must arrive within 10 s (`408`) and declare at most 1 MiB (`413`).

## Configuration

| Variable | Default | Meaning |
| --- | --- | --- |
| `PLUMED_RELAY_BIND` | `0.0.0.0:5174` | Listen address (set by `docker-compose.yml`) |
| `PLUMED_RELAY_TRUST_PROXY` | `false` | Take the client IP from the last `X-Forwarded-For` entry, only when the request comes from a loopback or private address (the reverse proxy). `docker-compose.yml` sets it to `true` |
| `PLUMED_RELAY_MAX_ROOMS` | `500` | Rooms open at the same time |
| `PLUMED_RELAY_ROOMS_PER_IP` | `100` | Rooms open at the same time for one client IP (IPv6: per /64), which also sets its share of requests in flight (4 per room). A hospital behind one NAT address shares it |
| `PLUMED_RELAY_ROOMS_PER_IP_PER_HOUR` | `300` | Room creations per client IP per hour |

The last two can be set in `.env` (copy `env.example`), `docker-compose.yml` passes them to the container.

## Develop

```sh
cargo test                                  # protocol + relay (build output goes to ../plumed_build_ignore_backups/)
node --test server/static-tests/*.test.mjs  # phone page
cargo run -p plumed-relay                   # http://localhost:5174
```

Browsers only give the microphone to secure origins, so testing on a real phone needs HTTPS (for example a reverse proxy with a certificate).

## Deploy

The relay runs behind a reverse proxy that terminates TLS. With Caddy:

```
turn.plumed.fr {
    reverse_proxy 127.0.0.1:5174
}
```

Keep Caddy's access log off (it is off unless a `log` directive is set): it would record client IPs and room ids, which the relay itself never logs.

Then on the server, in a copy of this directory: `cp env.example .env && sudo docker compose up -d --build`. The container is read-only, unprivileged (distroless, non-root, no capabilities) and checks its own health with `plumed-relay --healthcheck`.

Maintainers deploy with a local, gitignored `deploy.sh` that rsyncs this directory to the VPS and rebuilds the container.
