# ParakeetWeb remote microphone: protocol spec for a native relay-only receiver

Scope: what a native (Rust) receiver must do to replace the desktop browser tab of ParakeetWeb (PW) while talking to the unmodified PW signaling server and the unmodified phone page `remote-mic.html`, using only the HTTPS relay (WebSocket or long-poll), never WebRTC.

Written by Claude Code (read-only research of the sibling `parakeet_web` repo). All paths below are relative to the `parakeet_web` repo root. Abbreviations: `S` = `signaling/server.js`, `T` = `app/ui/src/lib/remote-relay-transport.js`, `W` = `app/ui/src/lib/remote-webrtc.js`, `H` = `app/ui/src/hooks/useRemoteMic.js`, `P` = `app/ui/src/remote-mic-entry.jsx`, `C` = `app/ui/src/lib/remote-crypto.js`, `K` = `app/ui/src/lib/remote-mic-handshake.js`.

## 1. Deployment

### 1.1 Origins and paths

- One public origin serves both the static app (incl. `/remote-mic.html`) and signaling. Caddy listens on `:5173` in the container (`docker/Caddyfile:11`).
- Browser-facing prefix is `/api/signal/*`. Caddy does `handle_path /api/signal/*` (strips the prefix) then `rewrite * /api{uri}` and proxies to `localhost:3001` (`docker/Caddyfile:269-283`). So public `https://HOST/api/signal/rooms` reaches the Node server as `/api/rooms`. All server routes below are given in the public form `/api/signal/...`; the server source uses `/api/...`.
- Caddy adds `Cache-Control: no-store`, `Pragma: no-cache`, `Vary: Origin, X-Room-Secret` on signaling (`Caddyfile` same block), and does not compress `/api/signal/*` (`Caddyfile:26-27`). It strips client `X-Forwarded-For`, `X-Real-IP`, `X-Forwarded-Host` (`Caddyfile` reverse_proxy block). Node trusts loopback proxy (`S:178`).
- WS upgrade goes through the same `handle_path` block (Caddy proxies upgrades transparently), server-side path regex `^/api/rooms/([A-Z0-9]{6})/relay$` (`S:1508`). Public URL: `wss://HOST/api/signal/rooms/<ROOM>/relay?secret=<SECRET>` (`T:26-36,64`).
- Server port default 3001 (`S:24`). `ALLOWED_ORIGINS` is required, comma list, each entry must start with `http://` or `https://` (`S:167-175`); plumbed in `docker/docker-compose.yml:139` and documented in `docker/env.example:216-219`.
- `RELAY_ENABLE` (server, default true, `S:101-131`) and `VITE_RELAY_ENABLE` (client, default enabled; phone/desktop skip relay only when `CONFIG.VITE_RELAY_ENABLE === 'false'`, `W:69`). Both are echoed by `docker/entrypoint.sh:292-293`. If either is false on the target deployment, a relay-only receiver cannot work.
- The phone page is `/remote-mic.html` (`app/ui/remote-mic.html`), loads `/config.js` then `/src/remote-mic-entry.jsx`. Room info is in the URL hash `#ROOM:SECRET` (never sent to the server). `?debug=1` loads eruda.

### 1.2 Origin checks per endpoint

- CORS on `/api/*` (`S:204-223`): if `Origin` is in the list, echo it in `Access-Control-Allow-Origin`; allow headers `Content-Type, X-Room-Secret, X-Slot-Token`; methods `GET, POST, OPTIONS`; `Max-Age 600`; OPTIONS -> 204. Irrelevant for a native client (no preflight).
- `validateOrigin` on every `/api` HTTP route (`S:228-263`):
  - `Origin` present: must be exactly in `ALLOWED_ORIGINS`, else 403 `{"error":"Forbidden","message":"Request origin not allowed"}`.
  - `Origin` absent: requires `Sec-Fetch-Site: same-origin`, else 403 `"Origin header required"`.
  - Native receiver MUST send `Origin: https://HOST` (the PW public origin, which is also where the QR points) on every HTTP request. Sending `Sec-Fetch-Site: same-origin` instead also works.
- WS upgrade (`S:1547-1555`, upgrade handler starts `S:1527`): if `Origin` present it must be allowed else `403`; if absent it is accepted. Secret authorises.
- JSON body limit 50 kB, parsed after origin check (`S:296`). Relay `up` uses a raw body limit 256 kB (`S:101-131`, `S:1151`).

### 1.3 Rate limits (per client IP, 60 s window, exceeding blocks for 60 s)

429 body `{"error":"Too many requests","retryAfter":N}` plus `Retry-After` header (`S:305-342,391-457`). `TEST_DISABLE_RATE_LIMIT=1` disables (`S:35`).

| Bucket | Limit/min | Used by |
|---|---|---|
| preflight | 200 | applied to every `/api` request before origin check, except relay up/down which use relayData (`S:280-288`) |
| roomCreation | 5 | `POST /rooms` |
| roomLookup | 30 | `GET /rooms/:id`, `GET /rooms/:id/offer` |
| general | 100 | offer/answer POST, answer GET, rearm, ICE, `relay/handshake`, `relay/close`, `stats`, AND each WS upgrade (`S:1560`) |
| iceConfig | 10 | `GET /config` |
| relayData | 15000 | `relay/up`, `relay/down` |

Note: Hospital NAT means the receiver and many other clients may share one public IP; 5 room creations/min per IP is the tightest limit.

### 1.4 Room id, secret, TTLs

- Room id: 6 chars from `ABCDEFGHJKLMNPQRSTUVWXYZ23456789` (`S:482-492`). Secret: 16 random bytes, base64url, 22 chars (`S:494-496`).
- `X-Room-Secret` header, constant-time compare; missing room OR bad secret -> 401 `{"error":"Invalid room secret"}` (`S:519-538`). Rooms that expired therefore return 401, not 404, on secret-gated routes.
- `ROOM_TTL` = 10 min from creation (`S:300`); `room.created` is set once (`S:927`) and never refreshed (not even by rearm, `S:1035-1046`). Cleanup runs every 60 s (`S:631-660`), deletes the room and closes relay slots (WS close 1001 `Room expired`, LP slots closed). So a relay session dies between 10 and 11 min after room creation, regardless of traffic. See Open questions.
- LP slot idle reap 60 s (`S:101-131`, teardown `S:690`). WS ping every 20 s, terminate if no pong (`S:789-799` area, interval `RELAY_WS_PING_INTERVAL_MS`).
- Node timeouts: `requestTimeout 35 s`, `headersTimeout 10 s`, `keepAliveTimeout 5 s` (`S:1493-1499`).

## 2. Full receiver sequence

Reference implementation of the receiver is the browser desktop (`H` + `W` offerer side). The phone is the WebRTC answerer (`W:600-655`).

### 2.1 Steps (MUST unless marked)

1. (MAY) `GET /api/signal/config` -> `{iceServers, dev, turnTimeout}` (`S:867-915`). Not needed relay-only.
2. `POST /api/signal/rooms` (no body needed) -> `200 {"roomId":"ABC234","secret":"<22 chars>"}` (`S:918-947`, client `W:446-453`).
3. Generate ECDH P-256 keypair, export raw public key base64 (section 3) (`H:322-323`).
4. `POST /api/signal/rooms/<ROOM>/offer`, headers `X-Room-Secret`, `Content-Type: application/json`, body `{"type":"offer","sdp":"<SDP>"}` (`S:950-956`, client `W:492-517`). Required because the phone refuses to join without it: `joinRoom` does `GET /rooms/:id`, throws `Room not ready yet` unless `hasOffer` (`W:603-616`), then `GET /offer` and `pc.setRemoteDescription(offer)` (`W:618-626`), which throws on an SDP the phone browser rejects. See 2.3 for the SDP.
5. Show QR: `https://HOST/remote-mic.html#<ROOM>:<SECRET>` (`H:764-765`). Phone parses it with `parseRemoteMicLink` (`app/ui/src/lib/remote-mic-link.js:1-49`): length <= 2048, same origin as the page, path ends in `/remote-mic.html`, hash split at the first `:`.
6. Wait for the phone: loop `GET /api/signal/rooms/<ROOM>/answer?wait=true` with `X-Room-Secret` (`S:965-1022`, client `W:522-593`). Server holds 30 s; `200` + answer JSON = phone joined; `204` = re-poll; `404` = room gone. Receiver never posts an answer and never needs to parse the phone's SDP. PW caps the total wait at 10 min (`W:55`).
7. Connect the relay (2.4). PW waits `_RELAY_GRACE_MS = 10000` after the answer before trying (`W:65,355-367`); a native receiver SHOULD connect immediately after step 6.
8. Send `{"type":"public-key","key":"<b64>"}` as a text frame (`H:772-775`). Server drops frames when the peer slot is empty (`S:762,801-804`), so see 2.5.
9. Handshake and audio per section 4.
10. On transport loss: rearm (2.6).

### 2.2 Phone side timing (fixed, cannot be changed from the receiver)

- `joinRoom` (`W:600-655`): GET room, GET offer, setRemoteDescription, `_fetchRemoteCandidates('offer')` (failure tolerated), createAnswer, `_waitForICE` (resolves on gathering complete or 5 s timeout, `W:731-760`), POST answer, `_startPolling('offer')`, `_startRelayRace()`.
- Relay grace: 10 s after the answer is posted (`W:65,355-367`), then WS first, HTTP long-poll second (`W:369-412`). Adoption closes the pc and fires `onConnected` (`W:414-438`).
- WebRTC connection timeout 15 s (or `turnTimeout*1000` from `/config`) (`W:47,104-106`); its disconnect is suppressed while relay is in flight or adopted (`W:693-706`).
- Phone `keyTimeout`: 30 s from start of `start()` waiting for `public-key`, then error + close (`P:653-663`). Expected phone time-to-relay ~= ICE gather (0 to 5 s) + 10 s grace + WS connect, so roughly 11 to 16 s after the answer. 30 s leaves margin but not much on slow networks.
- How to make the phone choose relay fast: not possible without modifying the phone. The grace is a hard-coded 10 s and is only skipped if WebRTC actually connects. The receiver can at least avoid adding latency: be on the relay before the phone arrives, and make WebRTC fail fast by giving the phone an offer with no candidates (it simply never connects).

### 2.3 The SDP offer (dummy, never used)

Server validator (`S:575-603`): `type` in `offer|answer`; `sdp` non-empty, <= 16 KiB, no NUL; after stripping one trailing `\r\n`, split on `\r\n|\n`; <= 200 lines; each <= 1024 bytes and matches `^[abceikmoprstuvz]=.*$` (so no empty lines, no trailing blank line); lines 0,1,2 start with `v=`, `o=`, `s=`.

Browser constraint: must be accepted by `setRemoteDescription` on Chrome/Safari/Firefox mobile, which requires ICE ufrag/pwd, a DTLS fingerprint, `a=setup:actpass`, and a data channel m-line (PW creates channel `remote-mic`, ordered, `W:492-517`). Suggested template (CRLF line endings, random ufrag/pwd per room, fingerprint may be random 32 bytes since DTLS never runs):

```
v=0
o=- <random u63> 2 IN IP4 127.0.0.1
s=-
t=0 0
a=group:BUNDLE 0
a=msid-semantic: WMS
m=application 9 UDP/DTLS/SCTP webrtc-datachannel
c=IN IP4 0.0.0.0
a=ice-ufrag:<4..32 ice-chars>
a=ice-pwd:<22..32 ice-chars>
a=ice-options:trickle
a=fingerprint:sha-256 <32 bytes as AA:BB:...>
a=setup:actpass
a=mid:0
a=sctp-port:5000
a=max-message-size:262144
```

No candidates are posted (do not call `/ice/offer`), so the phone's ICE never connects and it falls back to relay after the grace.

### 2.4 Relay transports

Slots: first claimer of a room's relay gets slot `a`, second `b`, third 409 (`S:1108-1135` LP, `S:1575-1579` WS). WS and LP slots can be mixed in one room (`deliverToPeer`, `S:801-825`). Frames are forwarded verbatim; binary flag preserved; text frames > 16 KiB -> WS close 4413 / LP 413; session total > 4 GiB -> 4413 / 413 (`S:733-783,1151-1193`).

WebSocket (preferred):

- `GET wss://HOST/api/signal/rooms/<ROOM>/relay?secret=<urlencoded SECRET>` with `Origin: https://HOST` (optional). Errors are raw HTTP before upgrade: 404 relay disabled/bad path, 403 origin, 429 + `Retry-After`, 401 bad room/secret, 409 slots full (`S:1527-1579`).
- Text frame = control JSON (`JSON.stringify(msg)`), binary frame = one encrypted audio packet (`T:120-165`). Receiver MUST answer pings (any RFC 6455 lib does).
- Close with 1000 `client closed`. Closing tears down the peer slot (`S:766-772`), so the phone sees a disconnect and starts its reconnect loop.
- The server never sends a "peer joined" event.

HTTP long-poll (fallback, all with `Origin`, `X-Room-Secret`):

- `POST /api/signal/rooms/<ROOM>/relay/handshake` (general bucket) -> `200 {"slot":"a"|"b","token":"<32 hex>"}`; 409 full; 404 relay disabled (`S:1108-1135`, client `T:244-273`).
- `POST .../relay/up` with `X-Slot-Token`; `Content-Type: application/octet-stream` = binary frame, anything else = text (PW sends `text/plain; charset=utf-8`). 204 ok; 409 `No relay session`; 401 `Invalid slot token`; 410 `Slot closed`; 413 too big (`S:1151-1193`, client `T:354-499`).
- `GET .../relay/down?wait=true` with `X-Slot-Token`: one frame per response; `200` with `Content-Type: application/octet-stream` (binary) or `text/plain; charset=utf-8` (text); `204` after 25 s; `429` + `Retry-After: 5` if > 4 waiters on the slot; `503` if > 10000 global waiters; `410` closed (`S:1196-1258`, client `T:275-344`). Peer queue holds 32 frames, drop-oldest (`S:101-131,804-808`), so keep a down request outstanding at all times.
- `POST .../relay/close` with `X-Slot-Token` (general bucket), idempotent 204, tears down peer (`S:1262-1274`, client `T:505-531`).
- PW client retry policy (`T:275-344,354-499`): down: network error retry every 2 s up to 5; 429/503 sleep `Retry-After` capped 30 s; 401/410 fatal. up: serial queue, max 200 queued binary frames, 5 attempts, backoff `200*2^(n-1)` ms capped 5 s, 401/410 fatal.

### 2.5 Getting `public-key` to the phone

Because frames to an empty slot are silently dropped (`S:762,801-804`) and there is no join notification, the receiver MUST either (a) resend `public-key` every ~1 to 2 s until `sender-public-key` arrives (the phone ignores duplicates once a handshake is in flight or bound, `P:670-690`, so this is safe), or (b) connect after the phone and send once. (a) is recommended; stop resending as soon as any frame arrives from the phone. Respect the phone's 30 s `keyTimeout`.

### 2.6 Close, rearm, reconnection

- PW receiver on disconnect: close transport, then `startRemoteMic({roomId, secret})` which does `adoptRoom` + `POST /rearm` (body `{}`) and reposts a fresh offer, keeping the same QR (`H:310-355`, `W:478-486`). Rearm clears offer, answer and ICE lists and wakes answer waiters, keeps relay state (`S:1035-1046`).
- Phone reconnect: up to 6 attempts, delays 1, 2, 4, 8, 8, 8 s, each re-running `joinRoom` on the same room (`P:214,377-395`); join errors during reconnect are expected and retried (`P:864-876`). A bound session resets the budget (`P:762-765`).
- Phone deliberate stop (`stop` message or user Disconnect) does not reconnect (`P:1186-1200`).
- Native receiver MUST close its relay slot (WS close or `/relay/close`) before rearming so the slot frees, then rearm, repost offer, wait for answer, reconnect relay, resend `public-key`, redo verification (a new phone keypair is generated every join, `P:670-697`).
- After the 10-min room TTL, rearm returns 401 and the receiver must create a new room and show a new QR.

### 2.7 Timers summary

| What | Value | Ref |
|---|---|---|
| Room TTL (hard, from creation) | 10 min (+ up to 60 s cleanup lag) | `S:300,631-660` |
| Answer long-poll hold | 30 s | `S:965-1022` |
| LP down hold | 25 s | `S:101-131` |
| LP idle slot reap | 60 s | `S:101-131` |
| WS ping | 20 s | `S:101-131` |
| Phone relay grace | 10 s | `W:65` |
| Phone ICE gather cap | 5 s | `W:731-760` |
| Phone WebRTC timeout | 15 s | `W:47` |
| Phone key timeout | 30 s | `P:653-663` |
| Verify peer-ack wait (both sides) | 60 s | `P:726`, `H:384-479` |
| Phone reconnect | 1,2,4,8,8,8 s | `P:214` |
| Phone audio batch | 500 ms | `P:208` |

## 3. Crypto (exact)

- Curve: ECDH P-256 (`C:16-23`).
- Public key wire format: raw uncompressed SEC1 point, 65 bytes `0x04||X||Y`, standard base64 with padding (RFC 4648 section 4, `+/`, `=`), always 88 chars (`C:30-33`).
- Import validation (receiver SHOULD mirror): string length <= 200, regex `^[A-Za-z0-9+/]+={0,2}$`, decodes to exactly 65 bytes, first byte `0x04` (`C:40-74`). Also reject points not on the curve (WebCrypto does).
- Shared secret: ECDH x-coordinate, 32 bytes (`deriveBits` 256) (`C:83-87`).
- KDF: HKDF-SHA256, IKM = shared secret, salt = UTF-8 `ParakeetWeb-RemoteMic-v1` (24 bytes), info = UTF-8 `AES-GCM-256-key` (15 bytes), L = 32 bytes -> AES-256-GCM key (`C:89-105`). Same key both directions.
- Packet: `IV(12 random bytes) || ciphertext || tag(16)`, no AAD (`C:114-125`), decrypt splits the same way (`C:133-142`). Overhead 28 bytes. No sequence numbers, no replay or reorder protection; ordering relies on the transport.
- Only binary audio frames are encrypted. All control messages (including `audio-config`, `stop`, `pause`, `verify-ok`) are plaintext JSON on the relay.
- Fingerprint: `SHA-256(receiverPubRaw65 || senderPubRaw65)`, hex uppercase, first 16 chars, grouped by 4 with `-`, e.g. `524C-F8E8-D5A4-2D82` (`C:172-191`). Receiver = desktop/native, sender = phone. Length: `computeFingerprintLength` always returns 16 (`C:216-218`); `K` fetches `/api/signal/stats` with a 2 s timeout and falls back to 16 (`K:19,31,43-58`), so the native receiver SHOULD hardcode 16. Role helper `computePairFingerprintForRole` (`K:73-80`), phone uses role `sender` (`P:704-705`).

## 4. Application protocol

All control messages are UTF-8 JSON objects with a `type` string, sent as text frames. Audio is binary frames.

### 4.1 Receiver -> phone

| Message | When | Phone behaviour | Ref |
|---|---|---|---|
| `{"type":"public-key","key":"<b64 88>"}` | after relay connects | generates its keypair, sends `sender-public-key`, shows fingerprint; duplicates ignored | `H:772-775`, `P:670-697` |
| `{"type":"verify-ok"}` | user confirmed fingerprint | resolves peer-ack or buffered if early | `H:384-479`, `P:769-787` |
| `{"type":"verify-deny"}` | user denied | aborts and closes; ignored after bound | `P:788-811` |
| `{"type":"audio-settings","noiseSuppression":bool,"autoGainControl":bool,"gain":number>=0}` | on bind and on change | stored, constraints apply on next capture, gain applied live; each field optional | `H:271-283`, `P:824-853` |
| `{"type":"stop-recording"}` | end current recording, keep link | phone flushes, sends `audio-end`, goes READY | `H:843`, `P:812-814,1137-1150` |
| `{"type":"stop"}` | full disconnect | flush, `audio-end`, cleanup, no reconnect | `H:854`, `P:815-817,1189-1200` |
| `{"type":"pause"}` / `{"type":"resume"}` | | suspend/resume AudioContext, phone replies `paused`/`resumed` | `H:872,879`, `P:818-823,1153-1184` |

Phone defaults before any `audio-settings`: noiseSuppression true, autoGainControl true, gain 2.0 (`P:191-195`). The receiver cannot start recording: the user taps Start on the phone.

### 4.2 Phone -> receiver

| Message | Notes | Ref |
|---|---|---|
| `{"type":"sender-public-key","key":"<b64 88>"}` | reply to `public-key` | `P:690-697` |
| `{"type":"verify-ok"}` / `{"type":"verify-deny"}` | phone user's decision | `P:713-721` |
| `{"type":"audio-config","sampleRate":int,"format":"pcm-s16"}` | start of every recording; `sampleRate` = phone `AudioContext.sampleRate`: 16000 when the browser reports a track rate (Chrome/Safari), else native, e.g. 44100/48000 (Firefox) | `P:903-925`, `app/ui/src/lib/audio.js:120-122` |
| `{"type":"audio-config","sampleRate":int,"format":"pcm-s16","source":"file"}` | saved-file send, rate = decoded rate (16000 requested, may be native) | `P:1065-1070` |
| binary | `encrypt(PCM batch)` | `P:308-333` |
| `{"type":"paused"}` / `{"type":"resumed"}` | | `P:1162,1183` |
| `{"type":"audio-end"}` | end of recording, after flushing the last partial batch | `P:1143-1146,1107` |

### 4.3 Handshake state machine (both sides symmetric)

1. Receiver sends `public-key`. Phone imports, derives, replies `sender-public-key`, computes fingerprint, shows modal.
2. Receiver on `sender-public-key`: import, derive, compute fingerprint, show modal (`H:384-479`). Receiver MUST NOT accept a second `sender-public-key` once a handshake is in flight.
3. Each side sends `verify-ok` (or `verify-deny` + close) on user decision, then waits up to 60 s for the peer's `verify-ok`. A peer `verify-ok` arriving before the local decision MUST be buffered (`P:769-787`, `H:480-521`).
4. Only after both `verify-ok`: bind the key. Before binding, the receiver ignores `audio-config`, `audio-end`, `paused`, `resumed` (`H:379-383`) and binary frames (`H:653-758`). After binding, `verify-deny` is ignored (`P:798-806`, desktop F-87).

### 4.4 Audio format

- Plaintext of each binary frame: mono PCM, `pcm-s16` = little-endian int16 (current phone always sends this), quantised `s<0 ? s*0x8000 : s*0x7FFF` (`P:266-273`). Legacy `pcm-f32` = little-endian float32, the default when `format` is absent (`H:522-598`).
- Receiver decode: s16 -> `v / 32768.0` (`H:653-758`); f32 must be finite.
- Batch ~500 ms per frame (`P:208,951`): at 16 kHz, 8000 samples = 16000 bytes plaintext, 16028 bytes on the wire.
- Recording = `audio-config`, N binary frames, `audio-end`. PW receiver concatenates, resamples to 16 kHz, makes a WAV, transcribes (`H:599-620`).
- PW receiver limits (SHOULD mirror): text frame > 4096 chars dropped (`H:364`); binary > 256 KiB dropped; total cap `10*60*96000` samples; `sampleRate` integer in 8000..96000; `format` in `pcm-f32|pcm-s16`; `source` in `mic|file`; a second `audio-config` within one recording is a protocol error that closes the session (`H:522-598,653-758`). Phone file cap 57.6 M samples (`P:228-233`).

## 5. MUST vs MAY for the native receiver

MUST:

- Send `Origin: https://HOST` (allowed origin) on all HTTP calls (`S:228-263`).
- Create a room, post a browser-acceptable SDP offer (2.3), show the QR with the same HOST.
- Long-poll `/answer?wait=true` (or otherwise not need it, see MAY) and connect to the relay with the room secret.
- Get `public-key` delivered after the phone attaches (2.5).
- Implement P-256 + HKDF + AES-GCM exactly as section 3, fingerprint order receiver||sender, 16 hex.
- Show the fingerprint and require user confirmation; implement `verify-ok`/`verify-deny` with early-arrival buffering and 60 s wait.
- Ignore audio and audio control before bind; ignore duplicate handshake messages after bind.
- Decode `pcm-s16` LE (and `pcm-f32` LE if `format` absent), resample from `sampleRate` to 16 kHz.
- Keep a long-poll `down` outstanding (LP) or answer WS pings.
- Free the relay slot on teardown (WS close or `/relay/close`).
- Handle the 10-min room expiry (new room + new QR).

MAY:

- Skip `/config`, `/stats`, ICE endpoints entirely.
- Skip waiting for `/answer`: just connect the relay right after posting the offer and resend `public-key` until answered (but answer polling gives a clean "phone joined" signal and costs one general-bucket request per 30 s).
- Send `audio-settings`, `pause`, `resume`, `stop-recording`, `stop`.
- Rearm and keep the QR on transport loss (PW behaviour) vs creating a new room.
- Use LP only (no WS) if WS is blocked by a proxy.

## 6. Test vectors

### 6.1 From PW unit tests (`test/unit/remote-crypto.test.mjs`)

- Round trip A->B decrypts (`:31`). Packet length for 4-byte plaintext = 12 + 4 + 16 = 32 (`:40`); IVs differ per call (`:42`).
- Tampered tag fails (`:49`); different key fails (`:56`).
- Exported pubkey base64 length 88, re-export identical (`:65,68`).
- Import rejects 201 chars (`/too long/`, `:72`), `not valid base64!!` (`:75`), `btoa('too short')` (`/expected 65-byte/`, `:78`).
- Fingerprint equal on both sides (`:87`), differs for different pairs (`:94`), default 16 hex matching `^[0-9A-F]{4}(-[0-9A-F]{4}){3}$` (`:100-101`), clamp to 16 and 64 (`:103,105`), non-numeric length -> 16 (`:115-119`), `computeFingerprintLength(1)` and `(100000)` = 16 (`:124-125`).
- Link parsing (`test/unit/remote-mic-link.test.mjs:14-88`): accepts same-origin and sub-path, secret is everything after the first `:`, trims whitespace; rejects other origin, `index.html`, `evil-remote-mic.html`, missing hash, no `:`, empty room or secret, non-strings, oversize.

The PW tests use random keys, so they contain no fixed vectors.

### 6.2 Deterministic vectors (generated for this spec with Node `crypto`, equivalent to WebCrypto)

```
receiver priv (scalar)  0101010101010101010101010101010101010101010101010101010101010101
sender   priv (scalar)  0202020202020202020202020202020202020202020202020202020202020202
receiver pub b64        BG/wO5SSQc4drdQ1GeaWDgqFtBppoFwygQOqK84VlMoWPE91OlW/AdxT9sCwx+7ni0DG/30lqW4igrmJzvccFEo=
sender   pub b64        BFUPRxAD89+Xw99QaseX9nIfsaH7e49vg9IkSYplyI4kE2CT1wEuUJpzcVy9CwCjzA/0tcAbP/oZarH7MnA2uOY=
ECDH shared x           eb459a6b30d70eb4c0ddcd34ea96ed59e8f8d1843c22dd895ab603aa13d9a81d
HKDF AES key            1d25d99afad8b6841cf1a308707af01be945ee32ab9b6f38de1d9236a66fb97b
fingerprint             524C-F8E8-D5A4-2D82
IV                      0a0a0a0a0a0a0a0a0a0a0a0a
plaintext (s16 LE: 0, 32767, -32768)  0000ff7f0080
packet (IV||ct||tag)    0a0a0a0a0a0a0a0a0a0a0a0a7e2407b137661362dde247091ee245e64899bf475033
```

Decoded plaintext samples via `/32768`: `[0.0, 0.999969482421875, -1.0]`.

## 7. Open questions / risks

1. Hard 10-min session cap on relay: rooms expire 10 to 11 min after creation even when active (`S:300,634,927`). In PW, WebRTC sessions survive room deletion but relay sessions are closed with 1001 `Room expired`; rearm then 401s. A long dictation over relay-only will be cut. Options: proactively create a new room (new QR, user rescans) before 10 min, or ask PW upstream to refresh `room.created` on relay activity/rearm.
2. Does every target phone browser accept the dummy SDP of 2.3 (Safari iOS especially)? Needs a real-device test. If `setRemoteDescription` throws, the phone shows an error and never reaches the relay.
3. With no remote candidates, could the phone's pc reach `failed` and fire `onDisconnected` via `_disconnectTimer` (`W:179-200`) before the 10 s grace expires? The 15 s connection timeout is gated on relay in flight (`W:693-706`), but the connectionstatechange path should be verified on Chrome and Safari.
4. Control messages are plaintext and unauthenticated: a malicious signaling server can inject `stop`, `pause`, `audio-config` (DoS, not confidentiality). Same as PW today.
5. No replay/reorder protection on audio (random IV, no counter): a malicious relay could replay or drop ciphertext frames. Same as PW.
6. Shared-IP rate limits behind hospital NAT (5 room creations/min, 100 general/min per IP) could be hit by several users.
7. Stale slots: if a previous session's LP slot is not closed, it occupies a slot for up to 60 s (idle reap) and new joins get 409. WS slots free on close. Verify behaviour when the phone reconnects while the old receiver WS is still half-open (20 s ping, terminate after a missed pong, so up to ~40 s).
8. Corporate proxies that terminate TLS may strip `Upgrade` (WS fails) or buffer long-poll responses; LP down 25 s holds may be cut by proxies with shorter idle timeouts (expect 204/timeouts, retry).
9. Line refs for `S:1108-1135` (handshake), `S:1527-1574` (upgrade) and the Caddyfile block were taken from the source at research time (PW `HEAD` not pinned); re-check if PW changes.
