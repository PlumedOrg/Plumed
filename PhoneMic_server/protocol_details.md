# Protocole PhoneMic de Plumed

Spécification du microphone distant de Plumed : le téléphone (page servie par le relais), le relais (`server/`) et l'application de bureau (`src-tauri/src/remote_mic/`). Le protocole est dérivé de celui de ParakeetWeb mais n'est pas compatible avec lui.

Sources de vérité, à modifier en même temps que ce document :

| Sujet | Rust | JavaScript (téléphone) |
| --- | --- | --- |
| Format filaire (enveloppes, `seq \|\| op \|\| payload`, opcodes) | `protocol/src/lib.rs` | `server/static/wire.js` |
| Cryptographie | `src-tauri/src/remote_mic/crypto.rs` | `server/static/crypto.js` |
| Appairage côté ordinateur (sans E/S) | `src-tauri/src/remote_mic/protocol.rs` | |
| Cycle de vie de la session côté ordinateur | `src-tauri/src/remote_mic/session.rs` | |
| Session côté téléphone | | `server/static/app.js`, `store.js` |

Les deux implémentations sont verrouillées par `protocol/test-vectors.json` (tests `cargo test` et `node --test server/static-tests/*.test.mjs`).

Les routes HTTP, les codes de réponse et les limites du relais sont décrits dans le [README](README.md#api-http) et ne sont pas répétés ici.

## 1. Salons et boîtes de réception

Un salon possède un secret par rôle (`desktop`, `phone`), transmis dans l'en-tête `x-room-secret`, et des boîtes de réception :

- `/api/rooms/{room}/desktop` : la boîte de l'ordinateur. Le téléphone y écrit (secret du téléphone), l'ordinateur la lit (secret de l'ordinateur).
- `/api/rooms/{room}/phone/{tag}` : une boîte par téléphone. `tag` est choisi au hasard par le téléphone (16 octets aléatoires en base64url, 22 caractères ; le relais accepte 16 à 43 caractères `[A-Za-z0-9_-]`). L'ordinateur y écrit, le téléphone la lit.

Règle générale : on lit sa propre boîte avec son propre secret, on écrit dans celle de l'autre avec le secret de l'autre côté. Le QR code ne contient que le secret du téléphone : qui le photographie ne peut ni lire la boîte de l'ordinateur ni fermer le salon.

Une lecture (`GET`) est un long-poll de 25 s qui renvoie tout ce qui est en attente sous forme d'une seule enveloppe, ou `204`.

Deux sortes de salons :

- le **salon d'accueil** (lobby), celui du QR code ;
- un **salon privé** par téléphone appairé, ouvert par l'ordinateur après confirmation.

## 2. Enveloppe

Corps de chaque `POST` et de chaque réponse `200` : des trames bout à bout, chacune `kind(u8) || len(u32 big-endian) || octets`.

- `kind = 0` : texte UTF-8 (JSON, uniquement les messages de poignée de main non authentifiés) ;
- `kind = 1` : binaire (un paquet chiffré).

Une trame fait au plus 256 Kio, un corps au plus 1 Mio. Deux enveloppes concaténées forment une enveloppe valide : le relais met les corps en file tels quels après validation.

Exemple : une trame texte `"hi"` suivie d'une trame binaire `[0xAB]` donne `00 00000002 6869 01 00000001 AB`.

## 3. Lien du QR code

```
https://turn.plumed.fr/#{room}.{secret du téléphone}.{clé publique de l'ordinateur}
```

La clé publique est un point P-256 non compressé (65 octets, préfixe `0x04`) encodé en base64url sans remplissage. Tout est dans le fragment (`#`), donc jamais envoyé au serveur. La page retire le fragment de la barre d'adresse dès qu'elle l'a lu.

## 4. Cryptographie

- Échange de clés : ECDH P-256. Chaque côté génère une paire éphémère ; côté ordinateur, une clé ne sert qu'à une seule poignée de main.
- Dérivation : HKDF-SHA256 avec
  - IKM = coordonnée x du secret partagé (32 octets),
  - sel = UTF-8 `Plumed-RemoteMic-v2`,
  - info = `label || clé publique de l'ordinateur (65) || clé publique du téléphone (65)`,
  - label = `phone-to-desktop` ou `desktop-to-phone`,
  - L = 32 octets.

  Une clé par sens, liée aux deux clés publiques : un paquet ne peut être ni renvoyé à son expéditeur ni rejoué dans un autre appairage.
- Paquet : `IV(12 octets aléatoires) || AES-256-GCM(clair) || tag(16)`.
- Code d'appairage : SHA-256(clé ordinateur || clé téléphone), 8 premiers octets en hexadécimal majuscule, groupés par 4 : `524C-F8E8-D5A4-2D82`.

## 5. Clair d'un paquet chiffré

```
seq(u64 big-endian) || op(u8) || payload
```

Chaque côté numérote ses paquets à partir de 1. Le récepteur ignore tout `seq` inférieur ou égal au dernier reçu (rejeu, réordonnancement). Côté ordinateur, un trou dans la numérotation (`seq != dernier + 1`) abandonne l'enregistrement en cours : un relais qui retiendrait un morceau ne peut pas faire transcrire une phrase amputée (un « pas » manquant inverse son sens).

Le téléphone conserve sa session dans IndexedDB ; il y enregistre une borne `seq` en avance de 2^20 sur le dernier numéro envoyé, pour qu'après un rechargement il ne réutilise jamais un numéro déjà émis.

### Opcodes

| op | Sens | Payload | Signification |
| --- | --- | --- | --- |
| `0x01` START | téléphone -> ordinateur | `sample_rate(u32 BE)`, 8000 à 96000 | Début d'un enregistrement. Un START sans END préalable abandonne l'enregistrement précédent |
| `0x02` AUDIO | téléphone -> ordinateur | PCM mono s16le | Échantillons de l'enregistrement en cours |
| `0x03` END | téléphone -> ordinateur | vide | Enregistrement terminé : à transcrire |
| `0x04` CANCEL | téléphone -> ordinateur | vide | Abandonner l'enregistrement en cours |
| `0x10` BOUND | ordinateur -> téléphone | `{room}.{secret du téléphone}` (ASCII) | Appairage confirmé ; désigne le salon privé où le téléphone doit se déplacer |
| `0x11` RECEIVED | ordinateur -> téléphone | vide | Enregistrement reçu, en cours de transcription |
| `0x12` DENIED | ordinateur -> téléphone | vide | Appairage refusé (ou laissé expirer) |
| `0x13` REPLACED | ordinateur -> téléphone | vide | Un autre téléphone a été appairé à la place |

## 6. Messages texte (non chiffrés)

Seuls deux messages circulent en clair, avant qu'une clé existe :

- téléphone -> ordinateur, dans la boîte de l'ordinateur du salon d'accueil :

  ```json
  {"type": "hello", "key": "<clé publique du téléphone>", "to": "<clé publique de l'ordinateur>", "inbox": "<tag>"}
  ```

  Les clés sont ici en base64 standard (65 octets, préfixe `0x04`). `to` indique quel QR code le téléphone a scanné.
- ordinateur -> téléphone :

  ```json
  {"type": "stale", "to": "<clé publique du téléphone>"}
  ```

  L'ordinateur n'a pas (ou plus) la clé privée correspondant à `to` : le QR code scanné est périmé. Seul le détenteur du secret de l'ordinateur peut écrire dans une boîte de téléphone, un tiers ne peut donc pas forger ce message.

## 7. Déroulement

1. L'ordinateur crée le salon d'accueil (`POST /api/rooms`), génère une paire de clés et affiche le QR code.
2. Le téléphone ouvre le lien, génère sa paire de clés et son `tag`, envoie `hello`, puis lit sa boîte `/phone/{tag}`. Il affiche le code d'appairage.
3. L'ordinateur reçoit le `hello` : le téléphone devient **candidat** (60 s pour confirmer) et le code d'appairage s'affiche. La clé du QR code étant consommée, une nouvelle paire est générée et le QR code change ; un `hello` adressé à l'ancienne clé reçoit `stale`.
4. L'utilisateur compare les deux codes et confirme sur l'ordinateur. L'ordinateur ouvre un salon privé et envoie, dans la boîte du téléphone du salon d'accueil, un BOUND chiffré qui le désigne (renvoyé toutes les 2 s, 5 fois, au cas où un vieil onglet lisant la même boîte avalerait le premier). En cas de refus ou d'expiration : DENIED.
5. Le téléphone se déplace dans le salon privé, où il écrit dans `/desktop` et lit `/phone/{tag}` avec le secret reçu. L'ordinateur remplace alors le salon d'accueil : la photo d'un ancien QR code n'ouvre plus rien, et quiconque l'a photographié ne peut au plus qu'encombrer le salon d'accueil (8 boîtes de téléphone), jamais le téléphone appairé.
6. Chaque enregistrement : START, AUDIO..., END (ou CANCEL). L'ordinateur répond RECEIVED puis transcrit et colle le texte.
7. Le QR code reste affiché : un nouveau téléphone qui le scanne et que l'utilisateur confirme remplace l'actuel, qui reçoit REPLACED.

## 8. Durées de vie

| Élément | Durée |
| --- | --- |
| Candidat non confirmé | 60 s |
| Appairage (ordinateur) et session enregistrée (téléphone) | 12 h |
| Salon sans polling (relais) | 10 min |
| Salon, quoi qu'il arrive (relais) | 24 h |

Si le relais supprime un salon (redémarrage, limite de 24 h), l'ordinateur ouvre un nouveau salon d'accueil, ou le téléphone doit scanner à nouveau. La session est supprimée côté téléphone à toute erreur définitive (salon fermé, `stale`, DENIED, REPLACED) et au scan d'un nouveau QR code.

## 9. Côté ordinateur : protections

- Au plus 4 enregistrements en attente de transcription ; au-delà, ils sont abandonnés.
- Un enregistrement encore en file après 60 s est abandonné.
- Une transcription prête plus de 20 s après la réception de END n'est pas collée (l'utilisateur est probablement passé à une autre fenêtre) : l'interface la propose à la place.
- Un enregistrement est limité à 20 minutes d'audio.

## 10. Modèle de menace

Le relais ne voit que des octets opaques et des identifiants aléatoires ; il garde tout en RAM et ne journalise ni IP ni identifiant de salon. Le chiffrement tient face à l'opérateur du relais et au réseau tant que la page du téléphone est honnête : c'est le même serveur qui sert la page et son code cryptographique, un relais compromis pourrait donc servir une page modifiée.
