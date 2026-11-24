# Microphone distant de ParakeetWeb : spécification du protocole pour un récepteur natif utilisant uniquement le relais

Périmètre : ce qu'un récepteur natif (Rust) doit faire pour remplacer l'onglet de navigateur de bureau de ParakeetWeb (PW) tout en dialoguant avec le serveur de signalisation PW non modifié et la page du téléphone `remote-mic.html` non modifiée, en n'utilisant que le relais HTTPS (WebSocket ou long-poll), jamais WebRTC.

Rédigé par Claude Code (étude en lecture seule du dépôt voisin `parakeet_web`). Tous les chemins ci-dessous sont relatifs à la racine du dépôt `parakeet_web`. Abréviations : `S` = `signaling/server.js`, `T` = `app/ui/src/lib/remote-relay-transport.js`, `W` = `app/ui/src/lib/remote-webrtc.js`, `H` = `app/ui/src/hooks/useRemoteMic.js`, `P` = `app/ui/src/remote-mic-entry.jsx`, `C` = `app/ui/src/lib/remote-crypto.js`, `K` = `app/ui/src/lib/remote-mic-handshake.js`.

## 1. Déploiement

### 1.1 Origines et chemins

- Une seule origine publique sert à la fois l'application statique (y compris `/remote-mic.html`) et la signalisation. Caddy écoute sur `:5173` dans le conteneur (`docker/Caddyfile:11`).
- Le préfixe exposé aux navigateurs est `/api/signal/*`. Caddy fait `handle_path /api/signal/*` (retire le préfixe) puis `rewrite * /api{uri}` et relaie vers `localhost:3001` (`docker/Caddyfile:269-283`). Ainsi, l'URL publique `https://HOST/api/signal/rooms` atteint le serveur Node sous la forme `/api/rooms`. Toutes les routes serveur ci-dessous sont données sous la forme publique `/api/signal/...` ; le code source du serveur utilise `/api/...`.
- Caddy ajoute `Cache-Control: no-store`, `Pragma: no-cache`, `Vary: Origin, X-Room-Secret` sur la signalisation (même bloc du `Caddyfile`), et ne compresse pas `/api/signal/*` (`Caddyfile:26-27`). Il retire les en-têtes client `X-Forwarded-For`, `X-Real-IP`, `X-Forwarded-Host` (bloc reverse_proxy du `Caddyfile`). Node fait confiance au proxy en loopback (`S:178`).
- L'upgrade WS passe par le même bloc `handle_path` (Caddy relaie les upgrades de façon transparente), regex de chemin côté serveur `^/api/rooms/([A-Z0-9]{6})/relay$` (`S:1508`). URL publique : `wss://HOST/api/signal/rooms/<ROOM>/relay?secret=<SECRET>` (`T:26-36,64`).
- Port du serveur par défaut 3001 (`S:24`). `ALLOWED_ORIGINS` est obligatoire, liste séparée par des virgules, chaque entrée doit commencer par `http://` ou `https://` (`S:167-175`) ; transmise dans `docker/docker-compose.yml:139` et documentée dans `docker/env.example:216-219`.
- `RELAY_ENABLE` (serveur, vrai par défaut, `S:101-131`) et `VITE_RELAY_ENABLE` (client, activé par défaut ; téléphone et ordinateur n'ignorent le relais que si `CONFIG.VITE_RELAY_ENABLE === 'false'`, `W:69`). Les deux sont affichés par `docker/entrypoint.sh:292-293`. Si l'un des deux est faux sur le déploiement cible, un récepteur utilisant uniquement le relais ne peut pas fonctionner.
- La page du téléphone est `/remote-mic.html` (`app/ui/remote-mic.html`), elle charge `/config.js` puis `/src/remote-mic-entry.jsx`. Les informations du salon sont dans le fragment d'URL `#ROOM:SECRET` (jamais envoyé au serveur). `?debug=1` charge eruda.

### 1.2 Vérifications d'origine par point d'accès

- CORS sur `/api/*` (`S:204-223`) : si `Origin` est dans la liste, il est renvoyé dans `Access-Control-Allow-Origin` ; en-têtes autorisés `Content-Type, X-Room-Secret, X-Slot-Token` ; méthodes `GET, POST, OPTIONS` ; `Max-Age 600` ; OPTIONS -> 204. Sans objet pour un client natif (pas de requête de pré-vérification).
- `validateOrigin` sur chaque route HTTP `/api` (`S:228-263`) :
  - `Origin` présent : doit figurer exactement dans `ALLOWED_ORIGINS`, sinon 403 `{"error":"Forbidden","message":"Request origin not allowed"}`.
  - `Origin` absent : exige `Sec-Fetch-Site: same-origin`, sinon 403 `"Origin header required"`.
  - Le récepteur natif DOIT envoyer `Origin: https://HOST` (l'origine publique de PW, qui est aussi celle vers laquelle pointe le QR) sur chaque requête HTTP. Envoyer `Sec-Fetch-Site: same-origin` à la place fonctionne aussi.
- Upgrade WS (`S:1547-1555`, le gestionnaire d'upgrade commence à `S:1527`) : si `Origin` est présent, il doit être autorisé, sinon `403` ; s'il est absent, la requête est acceptée. C'est le secret qui autorise.
- Limite de corps JSON de 50 ko, analysé après la vérification d'origine (`S:296`). Le `up` du relais utilise une limite de corps brut de 256 ko (`S:101-131`, `S:1151`).

### 1.3 Limites de débit (par IP cliente, fenêtre de 60 s, un dépassement bloque pendant 60 s)

Corps 429 `{"error":"Too many requests","retryAfter":N}` plus en-tête `Retry-After` (`S:305-342,391-457`). `TEST_DISABLE_RATE_LIMIT=1` les désactive (`S:35`).

| Compartiment | Limite/min | Utilisé par |
|---|---|---|
| preflight | 200 | appliqué à chaque requête `/api` avant la vérification d'origine, sauf relay up/down qui utilisent relayData (`S:280-288`) |
| roomCreation | 5 | `POST /rooms` |
| roomLookup | 30 | `GET /rooms/:id`, `GET /rooms/:id/offer` |
| general | 100 | POST d'offre/réponse, GET de réponse, rearm, ICE, `relay/handshake`, `relay/close`, `stats`, ET chaque upgrade WS (`S:1560`) |
| iceConfig | 10 | `GET /config` |
| relayData | 15000 | `relay/up`, `relay/down` |

Remarque : avec le NAT hospitalier, le récepteur et de nombreux autres clients peuvent partager une même IP publique ; 5 créations de salon/min par IP est la limite la plus serrée.

### 1.4 Identifiant de salon, secret, durées de vie

- Identifiant de salon : 6 caractères parmi `ABCDEFGHJKLMNPQRSTUVWXYZ23456789` (`S:482-492`). Secret : 16 octets aléatoires, base64url, 22 caractères (`S:494-496`).
- En-tête `X-Room-Secret`, comparaison en temps constant ; salon absent OU mauvais secret -> 401 `{"error":"Invalid room secret"}` (`S:519-538`). Les salons expirés renvoient donc 401, et non 404, sur les routes protégées par secret.
- `ROOM_TTL` = 10 min à partir de la création (`S:300`) ; `room.created` est défini une seule fois (`S:927`) et jamais rafraîchi (pas même par rearm, `S:1035-1046`). Le nettoyage tourne toutes les 60 s (`S:631-660`), supprime le salon et ferme les emplacements du relais (fermeture WS 1001 `Room expired`, emplacements LP fermés). Une session de relais meurt donc entre 10 et 11 min après la création du salon, quel que soit le trafic. Voir Questions ouvertes.
- Récupération des emplacements LP inactifs au bout de 60 s (`S:101-131`, démontage `S:690`). Ping WS toutes les 20 s, terminaison en l'absence de pong (zone `S:789-799`, intervalle `RELAY_WS_PING_INTERVAL_MS`).
- Délais Node : `requestTimeout 35 s`, `headersTimeout 10 s`, `keepAliveTimeout 5 s` (`S:1493-1499`).

## 2. Séquence complète du récepteur

L'implémentation de référence du récepteur est le navigateur de bureau (`H` + côté offrant de `W`). Le téléphone est le répondant WebRTC (`W:600-655`).

### 2.1 Étapes (obligatoires sauf mention contraire)

1. (FACULTATIF) `GET /api/signal/config` -> `{iceServers, dev, turnTimeout}` (`S:867-915`). Inutile en mode relais uniquement.
2. `POST /api/signal/rooms` (aucun corps nécessaire) -> `200 {"roomId":"ABC234","secret":"<22 chars>"}` (`S:918-947`, client `W:446-453`).
3. Générer une paire de clés ECDH P-256, exporter la clé publique brute en base64 (section 3) (`H:322-323`).
4. `POST /api/signal/rooms/<ROOM>/offer`, en-têtes `X-Room-Secret`, `Content-Type: application/json`, corps `{"type":"offer","sdp":"<SDP>"}` (`S:950-956`, client `W:492-517`). Obligatoire car le téléphone refuse de rejoindre sans elle : `joinRoom` fait `GET /rooms/:id`, lève `Room not ready yet` sauf si `hasOffer` (`W:603-616`), puis `GET /offer` et `pc.setRemoteDescription(offer)` (`W:618-626`), qui lève une exception sur un SDP que le navigateur du téléphone rejette. Voir 2.3 pour le SDP.
5. Afficher le QR : `https://HOST/remote-mic.html#<ROOM>:<SECRET>` (`H:764-765`). Le téléphone l'analyse avec `parseRemoteMicLink` (`app/ui/src/lib/remote-mic-link.js:1-49`) : longueur <= 2048, même origine que la page, chemin se terminant par `/remote-mic.html`, fragment coupé au premier `:`.
6. Attendre le téléphone : boucler sur `GET /api/signal/rooms/<ROOM>/answer?wait=true` avec `X-Room-Secret` (`S:965-1022`, client `W:522-593`). Le serveur tient 30 s ; `200` + JSON de réponse = le téléphone a rejoint ; `204` = relancer ; `404` = salon disparu. Le récepteur ne publie jamais de réponse et n'a jamais besoin d'analyser le SDP du téléphone. PW plafonne l'attente totale à 10 min (`W:55`).
7. Se connecter au relais (2.4). PW attend `_RELAY_GRACE_MS = 10000` après la réponse avant d'essayer (`W:65,355-367`) ; un récepteur natif DEVRAIT se connecter immédiatement après l'étape 6.
8. Envoyer `{"type":"public-key","key":"<b64>"}` en trame texte (`H:772-775`). Le serveur abandonne les trames lorsque l'emplacement du pair est vide (`S:762,801-804`), voir donc 2.5.
9. Échange de clés et audio selon la section 4.
10. En cas de perte du transport : rearm (2.6).

### 2.2 Chronologie côté téléphone (fixe, non modifiable depuis le récepteur)

- `joinRoom` (`W:600-655`) : GET du salon, GET de l'offre, setRemoteDescription, `_fetchRemoteCandidates('offer')` (échec toléré), createAnswer, `_waitForICE` (se résout à la fin de la collecte ou après 5 s, `W:731-760`), POST de la réponse, `_startPolling('offer')`, `_startRelayRace()`.
- Délai de grâce du relais : 10 s après la publication de la réponse (`W:65,355-367`), puis WS d'abord, long-poll HTTP ensuite (`W:369-412`). L'adoption ferme la pc et déclenche `onConnected` (`W:414-438`).
- Délai de connexion WebRTC de 15 s (ou `turnTimeout*1000` depuis `/config`) (`W:47,104-106`) ; sa déconnexion est ignorée tant que le relais est en cours ou adopté (`W:693-706`).
- `keyTimeout` du téléphone : 30 s à partir du début de `start()` en attente de `public-key`, puis erreur + fermeture (`P:653-663`). Le délai attendu avant que le téléphone passe sur le relais ~= collecte ICE (0 à 5 s) + 10 s de grâce + connexion WS, soit environ 11 à 16 s après la réponse. 30 s laisse de la marge, mais peu sur les réseaux lents.
- Comment faire choisir rapidement le relais au téléphone : impossible sans modifier le téléphone. Le délai de grâce est codé en dur à 10 s et n'est sauté que si WebRTC se connecte réellement. Le récepteur peut au moins éviter d'ajouter de la latence : être sur le relais avant l'arrivée du téléphone, et faire échouer WebRTC rapidement en donnant au téléphone une offre sans candidats (il ne se connecte simplement jamais).

### 2.3 L'offre SDP (factice, jamais utilisée)

Validateur du serveur (`S:575-603`) : `type` parmi `offer|answer` ; `sdp` non vide, <= 16 Kio, sans NUL ; après retrait d'un `\r\n` final, découpage sur `\r\n|\n` ; <= 200 lignes ; chacune <= 1024 octets et correspondant à `^[abceikmoprstuvz]=.*$` (donc pas de lignes vides, pas de ligne vide finale) ; les lignes 0, 1, 2 commencent par `v=`, `o=`, `s=`.

Contrainte navigateur : l'offre doit être acceptée par `setRemoteDescription` sur Chrome/Safari/Firefox mobile, ce qui exige ufrag/pwd ICE, une empreinte DTLS, `a=setup:actpass` et une ligne m de data channel (PW crée le canal `remote-mic`, ordonné, `W:492-517`). Modèle suggéré (fins de ligne CRLF, ufrag/pwd aléatoires par salon, l'empreinte peut être 32 octets aléatoires puisque DTLS ne s'exécute jamais) :

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

Aucun candidat n'est publié (ne pas appeler `/ice/offer`), l'ICE du téléphone ne se connecte donc jamais et il se rabat sur le relais après le délai de grâce.

### 2.4 Transports du relais

Emplacements : le premier à réclamer le relais d'un salon obtient l'emplacement `a`, le deuxième `b`, le troisième 409 (`S:1108-1135` LP, `S:1575-1579` WS). Des emplacements WS et LP peuvent être mélangés dans un même salon (`deliverToPeer`, `S:801-825`). Les trames sont transmises telles quelles ; l'indicateur binaire est préservé ; trames texte > 16 Kio -> fermeture WS 4413 / LP 413 ; total de session > 4 Gio -> 4413 / 413 (`S:733-783,1151-1193`).

WebSocket (à privilégier) :

- `GET wss://HOST/api/signal/rooms/<ROOM>/relay?secret=<urlencoded SECRET>` avec `Origin: https://HOST` (facultatif). Les erreurs sont du HTTP brut avant l'upgrade : 404 relais désactivé/mauvais chemin, 403 origine, 429 + `Retry-After`, 401 mauvais salon/secret, 409 emplacements pleins (`S:1527-1579`).
- Trame texte = JSON de contrôle (`JSON.stringify(msg)`), trame binaire = un paquet audio chiffré (`T:120-165`). Le récepteur DOIT répondre aux pings (toute bibliothèque RFC 6455 le fait).
- Fermer avec 1000 `client closed`. La fermeture démonte l'emplacement du pair (`S:766-772`), le téléphone voit donc une déconnexion et lance sa boucle de reconnexion.
- Le serveur n'envoie jamais d'événement « pair arrivé ».

Long-poll HTTP (solution de repli, toujours avec `Origin`, `X-Room-Secret`) :

- `POST /api/signal/rooms/<ROOM>/relay/handshake` (compartiment general) -> `200 {"slot":"a"|"b","token":"<32 hex>"}` ; 409 plein ; 404 relais désactivé (`S:1108-1135`, client `T:244-273`).
- `POST .../relay/up` avec `X-Slot-Token` ; `Content-Type: application/octet-stream` = trame binaire, toute autre valeur = texte (PW envoie `text/plain; charset=utf-8`). 204 ok ; 409 `No relay session` ; 401 `Invalid slot token` ; 410 `Slot closed` ; 413 trop gros (`S:1151-1193`, client `T:354-499`).
- `GET .../relay/down?wait=true` avec `X-Slot-Token` : une trame par réponse ; `200` avec `Content-Type: application/octet-stream` (binaire) ou `text/plain; charset=utf-8` (texte) ; `204` après 25 s ; `429` + `Retry-After: 5` si > 4 requêtes en attente sur l'emplacement ; `503` si > 10000 requêtes en attente au total ; `410` fermé (`S:1196-1258`, client `T:275-344`). La file du pair contient 32 trames, la plus ancienne étant abandonnée (`S:101-131,804-808`) : gardez donc en permanence une requête down en cours.
- `POST .../relay/close` avec `X-Slot-Token` (compartiment general), 204 idempotent, démonte le pair (`S:1262-1274`, client `T:505-531`).
- Politique de nouvelle tentative du client PW (`T:275-344,354-499`) : down : en cas d'erreur réseau, nouvelle tentative toutes les 2 s jusqu'à 5 fois ; 429/503 attente de `Retry-After` plafonnée à 30 s ; 401/410 fatals. up : file série, au plus 200 trames binaires en attente, 5 tentatives, attente `200*2^(n-1)` ms plafonnée à 5 s, 401/410 fatals.

### 2.5 Faire parvenir `public-key` au téléphone

Comme les trames vers un emplacement vide sont abandonnées silencieusement (`S:762,801-804`) et qu'il n'y a pas de notification d'arrivée, le récepteur DOIT soit (a) renvoyer `public-key` toutes les 1 à 2 s environ jusqu'à l'arrivée de `sender-public-key` (le téléphone ignore les doublons une fois un échange de clés en cours ou appairé, `P:670-690`, c'est donc sans risque), soit (b) se connecter après le téléphone et envoyer une seule fois. (a) est recommandé ; arrêter les renvois dès qu'une trame quelconque arrive du téléphone. Respecter le `keyTimeout` de 30 s du téléphone.

### 2.6 Fermeture, rearm, reconnexion

- Récepteur PW lors d'une déconnexion : fermer le transport, puis `startRemoteMic({roomId, secret})` qui fait `adoptRoom` + `POST /rearm` (corps `{}`) et republie une nouvelle offre, en gardant le même QR (`H:310-355`, `W:478-486`). Rearm efface l'offre, la réponse et les listes ICE et réveille les requêtes en attente de réponse, mais conserve l'état du relais (`S:1035-1046`).
- Reconnexion du téléphone : jusqu'à 6 tentatives, délais de 1, 2, 4, 8, 8, 8 s, chacune relançant `joinRoom` sur le même salon (`P:214,377-395`) ; les erreurs de jonction pendant la reconnexion sont attendues et retentées (`P:864-876`). Une session appairée réinitialise le budget (`P:762-765`).
- Un arrêt volontaire du téléphone (message `stop` ou bouton Déconnecter de l'utilisateur) ne déclenche pas de reconnexion (`P:1186-1200`).
- Le récepteur natif DOIT fermer son emplacement de relais (fermeture WS ou `/relay/close`) avant le rearm pour que l'emplacement se libère, puis faire le rearm, republier l'offre, attendre la réponse, se reconnecter au relais, renvoyer `public-key`, refaire la vérification (une nouvelle paire de clés du téléphone est générée à chaque jonction, `P:670-697`).
- Après la durée de vie de 10 min du salon, rearm renvoie 401 et le récepteur doit créer un nouveau salon et afficher un nouveau QR.

### 2.7 Récapitulatif des minuteurs

| Quoi | Valeur | Réf. |
|---|---|---|
| Durée de vie du salon (stricte, depuis la création) | 10 min (+ jusqu'à 60 s de retard du nettoyage) | `S:300,631-660` |
| Maintien du long-poll de réponse | 30 s | `S:965-1022` |
| Maintien du down LP | 25 s | `S:101-131` |
| Récupération d'emplacement LP inactif | 60 s | `S:101-131` |
| Ping WS | 20 s | `S:101-131` |
| Délai de grâce du relais (téléphone) | 10 s | `W:65` |
| Plafond de collecte ICE (téléphone) | 5 s | `W:731-760` |
| Délai WebRTC (téléphone) | 15 s | `W:47` |
| Délai d'attente de clé (téléphone) | 30 s | `P:653-663` |
| Attente du peer-ack de vérification (des deux côtés) | 60 s | `P:726`, `H:384-479` |
| Reconnexion du téléphone | 1,2,4,8,8,8 s | `P:214` |
| Lot audio du téléphone | 500 ms | `P:208` |

## 3. Cryptographie (exacte)

- Courbe : ECDH P-256 (`C:16-23`).
- Format filaire de la clé publique : point SEC1 brut non compressé, 65 octets `0x04||X||Y`, base64 standard avec remplissage (RFC 4648 section 4, `+/`, `=`), toujours 88 caractères (`C:30-33`).
- Validation à l'import (le récepteur DEVRAIT l'imiter) : longueur de chaîne <= 200, regex `^[A-Za-z0-9+/]+={0,2}$`, décodage en exactement 65 octets, premier octet `0x04` (`C:40-74`). Rejeter aussi les points hors de la courbe (WebCrypto le fait).
- Secret partagé : coordonnée x de l'ECDH, 32 octets (`deriveBits` 256) (`C:83-87`).
- KDF : HKDF-SHA256, IKM = secret partagé, sel = UTF-8 `ParakeetWeb-RemoteMic-v1` (24 octets), info = UTF-8 `AES-GCM-256-key` (15 octets), L = 32 octets -> clé AES-256-GCM (`C:89-105`). Même clé dans les deux sens.
- Paquet : `IV(12 random bytes) || ciphertext || tag(16)`, sans AAD (`C:114-125`), le déchiffrement découpe de la même façon (`C:133-142`). Surcoût de 28 octets. Pas de numéros de séquence, aucune protection contre le rejeu ou le réordonnancement ; l'ordre repose sur le transport.
- Seules les trames audio binaires sont chiffrées. Tous les messages de contrôle (y compris `audio-config`, `stop`, `pause`, `verify-ok`) sont du JSON en clair sur le relais.
- Empreinte : `SHA-256(receiverPubRaw65 || senderPubRaw65)`, hexadécimal en majuscules, 16 premiers caractères, groupés par 4 avec `-`, par exemple `524C-F8E8-D5A4-2D82` (`C:172-191`). Récepteur = ordinateur/natif, émetteur = téléphone. Longueur : `computeFingerprintLength` renvoie toujours 16 (`C:216-218`) ; `K` récupère `/api/signal/stats` avec un délai de 2 s et se rabat sur 16 (`K:19,31,43-58`), le récepteur natif DEVRAIT donc coder 16 en dur. Fonction utilitaire de rôle `computePairFingerprintForRole` (`K:73-80`), le téléphone utilise le rôle `sender` (`P:704-705`).

## 4. Protocole applicatif

Tous les messages de contrôle sont des objets JSON UTF-8 avec une chaîne `type`, envoyés en trames texte. L'audio est en trames binaires.

### 4.1 Récepteur -> téléphone

| Message | Quand | Comportement du téléphone | Réf. |
|---|---|---|---|
| `{"type":"public-key","key":"<b64 88>"}` | après la connexion au relais | génère sa paire de clés, envoie `sender-public-key`, affiche l'empreinte ; doublons ignorés | `H:772-775`, `P:670-697` |
| `{"type":"verify-ok"}` | l'utilisateur a confirmé l'empreinte | résout le peer-ack, ou mis en tampon s'il arrive tôt | `H:384-479`, `P:769-787` |
| `{"type":"verify-deny"}` | l'utilisateur a refusé | abandonne et ferme ; ignoré après appairage | `P:788-811` |
| `{"type":"audio-settings","noiseSuppression":bool,"autoGainControl":bool,"gain":number>=0}` | à l'appairage et à chaque changement | stocké, les contraintes s'appliquent à la capture suivante, le gain s'applique en direct ; chaque champ est facultatif | `H:271-283`, `P:824-853` |
| `{"type":"stop-recording"}` | terminer l'enregistrement en cours, garder la liaison | le téléphone vide son tampon, envoie `audio-end`, passe à READY | `H:843`, `P:812-814,1137-1150` |
| `{"type":"stop"}` | déconnexion complète | vidage, `audio-end`, nettoyage, pas de reconnexion | `H:854`, `P:815-817,1189-1200` |
| `{"type":"pause"}` / `{"type":"resume"}` | | suspend/reprend l'AudioContext, le téléphone répond `paused`/`resumed` | `H:872,879`, `P:818-823,1153-1184` |

Valeurs par défaut du téléphone avant tout `audio-settings` : noiseSuppression vrai, autoGainControl vrai, gain 2.0 (`P:191-195`). Le récepteur ne peut pas démarrer l'enregistrement : l'utilisateur touche Démarrer sur le téléphone.

### 4.2 Téléphone -> récepteur

| Message | Notes | Réf. |
|---|---|---|
| `{"type":"sender-public-key","key":"<b64 88>"}` | réponse à `public-key` | `P:690-697` |
| `{"type":"verify-ok"}` / `{"type":"verify-deny"}` | décision de l'utilisateur du téléphone | `P:713-721` |
| `{"type":"audio-config","sampleRate":int,"format":"pcm-s16"}` | début de chaque enregistrement ; `sampleRate` = `AudioContext.sampleRate` du téléphone : 16000 quand le navigateur indique la fréquence de la piste (Chrome/Safari), sinon la fréquence native, par exemple 44100/48000 (Firefox) | `P:903-925`, `app/ui/src/lib/audio.js:120-122` |
| `{"type":"audio-config","sampleRate":int,"format":"pcm-s16","source":"file"}` | envoi d'un fichier enregistré, fréquence = fréquence décodée (16000 demandé, peut être native) | `P:1065-1070` |
| binaire | `encrypt(PCM batch)` | `P:308-333` |
| `{"type":"paused"}` / `{"type":"resumed"}` | | `P:1162,1183` |
| `{"type":"audio-end"}` | fin de l'enregistrement, après l'envoi du dernier lot partiel | `P:1143-1146,1107` |

### 4.3 Machine à états de l'échange de clés (symétrique des deux côtés)

1. Le récepteur envoie `public-key`. Le téléphone importe, dérive, répond `sender-public-key`, calcule l'empreinte, affiche la fenêtre modale.
2. Le récepteur, à réception de `sender-public-key` : importe, dérive, calcule l'empreinte, affiche la fenêtre modale (`H:384-479`). Le récepteur NE DOIT PAS accepter un second `sender-public-key` une fois un échange de clés en cours.
3. Chaque côté envoie `verify-ok` (ou `verify-deny` + fermeture) selon la décision de l'utilisateur, puis attend jusqu'à 60 s le `verify-ok` du pair. Un `verify-ok` du pair arrivant avant la décision locale DOIT être mis en tampon (`P:769-787`, `H:480-521`).
4. Seulement après les deux `verify-ok` : lier la clé. Avant la liaison, le récepteur ignore `audio-config`, `audio-end`, `paused`, `resumed` (`H:379-383`) et les trames binaires (`H:653-758`). Après la liaison, `verify-deny` est ignoré (`P:798-806`, desktop F-87).

### 4.4 Format audio

- Texte clair de chaque trame binaire : PCM mono, `pcm-s16` = int16 little-endian (le téléphone actuel envoie toujours ce format), quantifié `s<0 ? s*0x8000 : s*0x7FFF` (`P:266-273`). Ancien format `pcm-f32` = float32 little-endian, par défaut quand `format` est absent (`H:522-598`).
- Décodage par le récepteur : s16 -> `v / 32768.0` (`H:653-758`) ; f32 doit être fini.
- Lot d'environ 500 ms par trame (`P:208,951`) : à 16 kHz, 8000 échantillons = 16000 octets en clair, 16028 octets sur le réseau.
- Enregistrement = `audio-config`, N trames binaires, `audio-end`. Le récepteur PW concatène, rééchantillonne à 16 kHz, produit un WAV, transcrit (`H:599-620`).
- Limites du récepteur PW (DEVRAIT les imiter) : trame texte > 4096 caractères abandonnée (`H:364`) ; binaire > 256 Kio abandonné ; plafond total de `10*60*96000` échantillons ; `sampleRate` entier dans 8000..96000 ; `format` parmi `pcm-f32|pcm-s16` ; `source` parmi `mic|file` ; un second `audio-config` au sein d'un même enregistrement est une erreur de protocole qui ferme la session (`H:522-598,653-758`). Plafond fichier du téléphone : 57,6 M échantillons (`P:228-233`).

## 5. Obligatoire et facultatif pour le récepteur natif

OBLIGATOIRE :

- Envoyer `Origin: https://HOST` (origine autorisée) sur tous les appels HTTP (`S:228-263`).
- Créer un salon, publier une offre SDP acceptable par un navigateur (2.3), afficher le QR avec le même HOST.
- Faire le long-poll `/answer?wait=true` (ou s'en passer, voir FACULTATIF) et se connecter au relais avec le secret du salon.
- Faire parvenir `public-key` après l'arrivée du téléphone (2.5).
- Implémenter P-256 + HKDF + AES-GCM exactement comme en section 3, empreinte dans l'ordre receiver||sender, 16 hex.
- Afficher l'empreinte et exiger la confirmation de l'utilisateur ; implémenter `verify-ok`/`verify-deny` avec mise en tampon des arrivées précoces et attente de 60 s.
- Ignorer l'audio et les contrôles audio avant la liaison ; ignorer les messages d'échange de clés en double après la liaison.
- Décoder `pcm-s16` LE (et `pcm-f32` LE si `format` est absent), rééchantillonner de `sampleRate` à 16 kHz.
- Garder un `down` en long-poll en cours (LP) ou répondre aux pings WS.
- Libérer l'emplacement du relais au démontage (fermeture WS ou `/relay/close`).
- Gérer l'expiration du salon à 10 min (nouveau salon + nouveau QR).

FACULTATIF :

- Ignorer complètement `/config`, `/stats` et les points d'accès ICE.
- Ne pas attendre `/answer` : se connecter au relais juste après la publication de l'offre et renvoyer `public-key` jusqu'à obtenir une réponse (mais le polling de réponse donne un signal propre « le téléphone a rejoint » et coûte une requête du compartiment general toutes les 30 s).
- Envoyer `audio-settings`, `pause`, `resume`, `stop-recording`, `stop`.
- Faire un rearm et garder le QR en cas de perte du transport (comportement de PW) plutôt que de créer un nouveau salon.
- N'utiliser que le LP (pas de WS) si un proxy bloque les WS.

## 6. Vecteurs de test

### 6.1 Issus des tests unitaires de PW (`test/unit/remote-crypto.test.mjs`)

- L'aller-retour A->B se déchiffre (`:31`). Longueur de paquet pour un texte clair de 4 octets = 12 + 4 + 16 = 32 (`:40`) ; les IV diffèrent à chaque appel (`:42`).
- Un tag altéré échoue (`:49`) ; une clé différente échoue (`:56`).
- La clé publique exportée fait 88 caractères en base64, une réexportation est identique (`:65,68`).
- L'import rejette 201 caractères (`/too long/`, `:72`), `not valid base64!!` (`:75`), `btoa('too short')` (`/expected 65-byte/`, `:78`).
- Empreinte identique des deux côtés (`:87`), différente pour des paires différentes (`:94`), 16 hex par défaut correspondant à `^[0-9A-F]{4}(-[0-9A-F]{4}){3}$` (`:100-101`), bornée à 16 et 64 (`:103,105`), longueur non numérique -> 16 (`:115-119`), `computeFingerprintLength(1)` et `(100000)` = 16 (`:124-125`).
- Analyse du lien (`test/unit/remote-mic-link.test.mjs:14-88`) : accepte la même origine et un sous-chemin, le secret est tout ce qui suit le premier `:`, supprime les espaces ; rejette une autre origine, `index.html`, `evil-remote-mic.html`, un fragment absent, l'absence de `:`, un salon ou un secret vide, les non-chaînes, une taille excessive.

Les tests de PW utilisent des clés aléatoires, ils ne contiennent donc aucun vecteur fixe.

### 6.2 Vecteurs déterministes (générés pour cette spécification avec `crypto` de Node, équivalent à WebCrypto)

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

Échantillons du texte clair décodés via `/32768` : `[0.0, 0.999969482421875, -1.0]`.

## 7. Questions ouvertes / risques

1. Plafond strict de 10 min par session sur le relais : les salons expirent 10 à 11 min après leur création même s'ils sont actifs (`S:300,634,927`). Dans PW, les sessions WebRTC survivent à la suppression du salon, mais les sessions de relais sont fermées avec 1001 `Room expired` ; le rearm renvoie alors 401. Une longue dictée en relais uniquement sera coupée. Options : créer proactivement un nouveau salon (nouveau QR, l'utilisateur rescanne) avant 10 min, ou demander à PW en amont de rafraîchir `room.created` lors d'une activité sur le relais ou d'un rearm.
2. Tous les navigateurs de téléphone ciblés acceptent-ils le SDP factice de 2.3 (Safari iOS en particulier) ? Un test sur appareil réel est nécessaire. Si `setRemoteDescription` lève une exception, le téléphone affiche une erreur et n'atteint jamais le relais.
3. Sans candidats distants, la pc du téléphone pourrait-elle atteindre `failed` et déclencher `onDisconnected` via `_disconnectTimer` (`W:179-200`) avant l'expiration des 10 s de grâce ? Le délai de connexion de 15 s est conditionné au relais en cours (`W:693-706`), mais le chemin connectionstatechange doit être vérifié sur Chrome et Safari.
4. Les messages de contrôle sont en clair et non authentifiés : un serveur de signalisation malveillant peut injecter `stop`, `pause`, `audio-config` (déni de service, pas d'atteinte à la confidentialité). Identique à PW aujourd'hui.
5. Aucune protection contre le rejeu ou le réordonnancement de l'audio (IV aléatoire, pas de compteur) : un relais malveillant pourrait rejouer ou abandonner des trames chiffrées. Identique à PW.
6. Les limites de débit par IP partagée derrière un NAT hospitalier (5 créations de salon/min, 100 general/min par IP) pourraient être atteintes par plusieurs utilisateurs.
7. Emplacements périmés : si l'emplacement LP d'une session précédente n'est pas fermé, il occupe un emplacement jusqu'à 60 s (récupération des inactifs) et les nouvelles jonctions reçoivent 409. Les emplacements WS se libèrent à la fermeture. Vérifier le comportement quand le téléphone se reconnecte alors que l'ancien WS du récepteur est encore à moitié ouvert (ping de 20 s, terminaison après un pong manqué, donc jusqu'à ~40 s).
8. Les proxys d'entreprise qui terminent le TLS peuvent retirer `Upgrade` (échec des WS) ou mettre en tampon les réponses de long-poll ; les maintiens de 25 s du down LP peuvent être coupés par des proxys aux délais d'inactivité plus courts (attendez-vous à des 204/délais dépassés, et retentez).
9. Les références de lignes pour `S:1108-1135` (handshake), `S:1527-1574` (upgrade) et le bloc du Caddyfile ont été relevées dans le code source au moment de l'étude (`HEAD` de PW non figé) ; à revérifier si PW change.
