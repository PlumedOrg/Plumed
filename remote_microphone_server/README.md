# Plumed PhoneMic relay (alpha)

> **Alpha:** PhoneMic (the remote microphone) is a pre-release feature, expect bugs and breaking protocol changes between Plumed versions.

This directory holds everything that runs on the server side of Plumed: the remote microphone relay, a small Rust (axum) HTTPS relay that also serves the phone page, and, as a second independent service, the [model relay](#model-relay) that streams the speech models from Hugging Face. It was written with Claude Code.

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
| `server/src/serve.rs` | Serving helpers shared by both binaries (environment, client IP behind the proxy, shutdown, `--healthcheck`) |
| `protocol_details.md` | Detailed spec of the ParakeetWeb remote microphone protocol (relay-only transport, crypto handshake, framing) that the Plumed protocol was derived from |
| `models/` | The model relay `plumed-models` (see [Model relay](#model-relay)) |
| `Dockerfile`, `docker-compose.yml`, `env.example` | Deployment of both services (Dockerfile targets `relay` and `models`) |

The phone's `wire.js` and `crypto.js` mirror the Rust code (`protocol/src/lib.rs`, `src-tauri/src/remote_mic/crypto.rs`). This duplication across languages is unavoidable, so both sides are pinned by `protocol/test-vectors.json`: change them together.

## Phone page features

- Manual mode: tap to start and stop, or hold the button for push-to-talk.
- Hands-free mode: the phone listens continuously and only sends speech. An energy VAD with an adaptive noise floor (20 ms frames) opens an utterance after 60 ms of speech, includes the 400 ms heard before it, closes it after 900 ms of silence, and drops it when it held less than 250 ms of speech. Silence never leaves the phone.
- Pause and resume, sending an audio file (20 minutes max).
- Screen wake lock while connected, so the phone does not fall asleep.
- Reconnects by itself after a network loss. The room secret is removed from the address bar once read. Once bound, the phone keeps its session in IndexedDB (non-extractable AES keys, private room, sequence counters) for 12 h, the lifetime of the binding on the desktop: a page reload, or the phone killing the tab in the background, picks the session up again without a new scan. The record is deleted on any terminal error (room closed, phone replaced, pairing denied) and when a new QR code is scanned.

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

The last three can be set in `.env` (copy `env.example`), `docker-compose.yml` passes them to the container.

## Model relay

Plumed does not bundle its speech model: on first launch the app downloads the model the user picks from Hugging Face. Hospital networks often block huggingface.co but let plumed.fr through, so the app falls back to `plumed-models`, a second, independent service of this directory: a Rust port of ParakeetWeb's HuggingFace relay (`signaling/hf-relay.js`), written with Claude Code.

- `GET` / `HEAD /models/<owner>/<name>/<path>?revision=<commit>` answers the file of that repository at that commit (the scheme lives in `protocol/src/models.rs`, `relay_path`, used by the app too). `GET /healthz` answers `ok`.
- It streams from huggingface.co without writing to disk and without buffering beyond the stream's backpressure window (constant memory, the cost is bandwidth only). `Range`, `If-Range` and `If-None-Match` are forwarded; `Content-Length`, `Content-Range`, `Accept-Ranges` and `ETag` come back, so the app resumes an interrupted download. `HEAD` stops at the hub (`X-Linked-Size`, `X-Linked-Etag`) and never touches the CDN.
- Never an open proxy: only the allowlisted `repo@revision` pairs are served, anything else is a `404` without contacting the hub; every path segment must be `[A-Za-z0-9._-]+` (no `..`, no percent-encoding); redirects are followed by hand, at most 5, and each hop must be https on `huggingface.co`, `*.huggingface.co` or `*.hf.co`, otherwise `502`.
- The optional `HF_TOKEN` is sent to huggingface.co only, never to its CDN.
- Concurrent file streams are capped for everyone and per client IP (`503` with `Retry-After: 5`). Upstream headers must arrive within 30 s (`504`), a body that sends nothing for 60 s is cut. Client IPs are never logged; errors log the repository and file only.

The allowlist defaults to `MODEL_REPOS` in `protocol/src/models.rs`, the same list the app's catalog is built on (one source of truth): to offer a new model or revision, update that list and the catalog (`src-tauri/src/model/catalog.rs`), then redeploy. Keep the old revision in the list while released apps still download it.

| Variable | Default | Meaning |
| --- | --- | --- |
| `PLUMED_MODELS_BIND` | `0.0.0.0:5175` | Listen address (set by `docker-compose.yml`) |
| `PLUMED_MODELS_TRUST_PROXY` | `false` | Same as `PLUMED_RELAY_TRUST_PROXY`, `docker-compose.yml` sets it to `true` |
| `PLUMED_MODELS_REPOS` | the app's repositories | Comma-separated `owner/name@revision` pairs served; a malformed entry stops the boot |
| `PLUMED_MODELS_MAX_STREAMS` | `32` | File streams at the same time |
| `PLUMED_MODELS_STREAMS_PER_IP` | `8` | File streams at the same time per client IP (IPv6: per /64) |
| `HF_TOKEN` | empty | Hugging Face token, only needed if the hub rate-limits the server |

All but the bind address can be set in `.env`, `docker-compose.yml` passes them to the container.

### App releases

The same service also relays the app's own releases, so the updater works where github.com is blocked (`models/src/releases.rs`, scheme in `protocol/src/releases.rs`, shared with the app; written with Claude Code). The app asks GitHub first and only uses these routes when that fails:

- `GET /releases/latest/download/latest.json`: the latest stable release's updater manifest, from `https://github.com/PlumedOrg/Plumed/releases/latest/download/latest.json`.
- `GET /releases/download/<tag>/<asset>`: one asset of one release, from `https://github.com/PlumedOrg/Plumed/releases/download/<tag>/<asset>`. The app rewrites the installer URL found in `latest.json` to this route when it fell back to the relay.
- `GET /releases/list`: the 10 newest releases (`https://api.github.com/repos/PlumedOrg/Plumed/releases?per_page=10`), read by the beta channel. Kept in memory 5 minutes (at most 12 GitHub API calls an hour, the unauthenticated limit is 60), one fetch at a time, at most 4 MiB.

Same hardening as the model routes: only the `PlumedOrg/Plumed` repository (it is not part of the path, so no other one can be asked for), `<tag>` and `<asset>` must be `[A-Za-z0-9._-]+` tokens, no query string, anything else is a `404` without contacting GitHub. Redirects are followed by hand, at most 5, each hop https on `github.com`, `objects.githubusercontent.com` or `release-assets.githubusercontent.com` (otherwise `502`). Files are streamed without buffering, `Range` / `If-Range` / `If-None-Match` pass through, and release streams share the global and per IP caps of the model streams (the listing takes a per IP slot too). The `HF_TOKEN` never goes to GitHub.

A relay cannot forge an update: the app installs only an installer whose minisign signature matches the updater public key compiled into it (`plugins.updater.pubkey` in `src-tauri/tauri.conf.json`), and the private key never leaves the maintainer's machine and the GitHub secrets. A compromised relay could at most withhold updates or serve an older signed version, which the app refuses to install since it never downgrades.

## Develop

```sh
cargo test                                  # protocol + relay (build output goes to ../plumed_build_ignore_backups/)
node --test server/static-tests/*.test.mjs  # phone page
cargo run -p plumed-relay                   # http://localhost:5174
cargo run -p plumed-models                  # http://localhost:5175 (model relay)
```

`cargo test` also runs the model relay against a fake Hugging Face (hub redirecting to a CDN) and a fake GitHub over real sockets (`models/tests/relay.rs`).

Browsers only give the microphone to secure origins, so testing on a real phone needs HTTPS (for example a reverse proxy with a certificate).

## Deploy

The relay runs behind a reverse proxy that terminates TLS. With Caddy:

```
turn.plumed.fr {
    reverse_proxy 127.0.0.1:5174
}
```

The model relay needs its own block. `flush_interval -1` sends each chunk as it arrives (no buffering of a 650 MB file), and there must be no `encode` directive on it: compression would drop `Content-Length` and break `Range` resumption:

```
models.plumed.fr {
    reverse_proxy 127.0.0.1:5175 {
        flush_interval -1
    }
}
```

Keep Caddy's access log off on both (it is off unless a `log` directive is set): it would record client IPs and room ids or downloaded files, which the relays themselves never log.

Then on the server, in a copy of this directory: `cp env.example .env && sudo docker compose up -d --build`. Both containers are read-only, unprivileged (distroless, non-root, no capabilities) and check their own health with `plumed-relay --healthcheck` and `plumed-models --healthcheck`.

Maintainers deploy with a local, gitignored `deploy.sh` that rsyncs this directory to the VPS, builds both images, swaps both containers and checks `https://turn.plumed.fr/healthz` and `https://models.plumed.fr/healthz`.
