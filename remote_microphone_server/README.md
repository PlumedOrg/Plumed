# Plumed remote microphone relay (alpha)

> **Alpha:** the remote microphone is a pre-release feature, expect bugs and breaking protocol changes between Plumed versions.

This directory holds everything that runs on the server side of the Plumed remote microphone: a small Rust (axum) HTTPS relay that also serves the phone page. It was written with Claude Code.

How it works:

1. Plumed (desktop) creates a room on the relay and shows a QR code: `https://turn.plumed.fr/#{room}.{secret}.{desktop public key}`.
2. The phone opens the page, generates its own key and posts a `hello` with its public key. Both screens show the same pairing code (for example `524C-F8E8-D5A4-2D82`) and the desktop user confirms it.
3. From then on every audio frame is encrypted end to end (ECDH P-256, HKDF-SHA256, AES-256-GCM). The relay only forwards opaque bytes between two mailboxes: it cannot hear anything, keeps everything in RAM, and logs no IP or room id.

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
- Reconnects by itself after a network loss or a page reload, without a new pairing (the session key stays in `sessionStorage` of that tab only).

## HTTP API

| Request | Answer |
| --- | --- |
| `POST /api/rooms` | `201 {"room","secret"}`, `429` (per-IP limit) or `503` (server full) |
| `POST /api/rooms/{room}/{desktop\|phone}` + `x-room-secret` | Appends an envelope to the other side's mailbox: `204`, `400` (malformed), `429` (mailbox full, 4 MiB) |
| `GET /api/rooms/{room}/{desktop\|phone}` + `x-room-secret` | Long-poll (25 s): `200` with the queued envelopes, `204` if nothing came, `404` once the room is closed |
| `DELETE /api/rooms/{room}` + `x-room-secret` | `204` |
| `GET /healthz` | `ok` |

Unknown room and wrong secret both answer `404`. Rooms are dropped after 10 minutes without polling or 24 hours of age.

## Configuration

| Variable | Default | Meaning |
| --- | --- | --- |
| `PLUMED_RELAY_BIND` | `0.0.0.0:5174` | Listen address (set by `docker-compose.yml`) |
| `PLUMED_RELAY_TRUST_PROXY` | `false` | Take the client IP from the last `X-Forwarded-For` entry. `docker-compose.yml` sets it to `true`: only enable it behind a reverse proxy |
| `PLUMED_RELAY_MAX_ROOMS` | `500` | Rooms open at the same time |
| `PLUMED_RELAY_ROOMS_PER_IP_PER_HOUR` | `60` | Room creations per client IP per hour |

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

Then on the server, in a copy of this directory: `cp env.example .env && sudo docker compose up -d --build`. The container is read-only, unprivileged (distroless, non-root, no capabilities) and checks its own health with `plumed-relay --healthcheck`.

Maintainers deploy with a local, gitignored `deploy.sh` that rsyncs this directory to the VPS and rebuilds the container.
