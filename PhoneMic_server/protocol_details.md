# Protocole PhoneMic de Plumed

Spécification du microphone distant de Plumed : le téléphone (page servie par le relais), le relais (`server/`) et l'application de bureau (`src-tauri/src/remote_mic/`). Le protocole est dérivé de celui de ParakeetWeb mais n'est pas compatible avec lui.

Sources de vérité, à modifier en même temps que ce document :

| Sujet | Rust | JavaScript (téléphone) |
| --- | --- | --- |
| Format filaire (enveloppes, `seq \|\| op \|\| payload`, opcodes) | `protocol/src/lib.rs` | `server/static/wire.js` |
| Cryptographie | `src-tauri/src/remote_mic/crypto.rs` | `server/static/crypto.js` |
| Appairage côté ordinateur (sans E/S) | `src-tauri/src/remote_mic/protocol.rs` | |
| Cycle de vie de la session côté ordinateur | `src-tauri/src/remote_mic/session.rs` | |
| Session côté téléphone | | `server/static/app.js`, `store.js`, `outbox.js` |

Les deux implémentations sont verrouillées par `protocol/test-vectors.json` (tests `cargo test` et `node --test server/static-tests/*.test.mjs`).

Les routes HTTP, les codes de réponse et les limites du relais sont décrits dans le [README](README.md#api-http) et ne sont pas répétés ici.

## 0. Version

Le protocole entier (format filaire, cryptographie, appairage) porte une version, `PROTOCOL_VERSION` dans `protocol/src/lib.rs` (actuellement `5.0.0`), reprise dans `wire.js` et dans `test-vectors.json`.

- Le relais la sert sur `GET /api/version` : `{"protocol": "5.0.0"}`.
- Avant d'ouvrir le moindre salon, Plumed la compare à la sienne et refuse toute différence (ou un `404`, relais antérieur au versionnage) avec le message : « Le relais utilise le protocole de chiffrement vX.Y.Z et notre version est vA.B.C : impossible de l'utiliser. La rétrocompatibilité du protocole n'est volontairement pas prise en charge, pour réduire la surface d'attaque. »
- Elle fait partie du sel HKDF (section 4) : même un pair d'une autre version qui passerait la vérification ne pourrait rien déchiffrer.

Toute modification du protocole doit incrémenter la version et régénérer les vecteurs de test : `node protocol/gen-test-vectors.mjs protocol/test-vectors.json <version>`.

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
https://turn.plumed.fr/#{room}.{secret du téléphone}.{clé publique de l'ordinateur}.{secret d'appairage}
```

La clé publique est un point P-256 non compressé (65 octets, préfixe `0x04`), le secret d'appairage 32 octets aléatoires, tous deux encodés en base64url sans remplissage. Le secret d'appairage change en même temps que la clé. Tout est dans le fragment (`#`), donc jamais envoyé au serveur. La page retire le fragment de la barre d'adresse dès qu'elle l'a lu.

## 4. Cryptographie

- Échange de clés : ECDH P-256. Chaque côté génère une paire éphémère ; côté ordinateur, une clé ne sert qu'à une seule poignée de main.
- Dérivation : HKDF-SHA256 avec
  - IKM = coordonnée x du secret partagé (32 octets) `||` secret d'appairage du QR code (32 octets),
  - sel = UTF-8 `Plumed-RemoteMic-{version}`, par exemple `Plumed-RemoteMic-5.0.0`,
  - info = `label || clé publique de l'ordinateur (65) || clé publique du téléphone (65)`,
  - label = `phone-to-desktop`, `desktop-to-phone` ou `hello-proof`,
  - L = 32 octets.

  Une clé par sens, liée aux deux clés publiques : un paquet ne peut être ni renvoyé à son expéditeur ni rejoué dans un autre appairage. Le label `hello-proof` donne la preuve du `hello` (section 6).
- Paquet : `IV(12 octets aléatoires) || AES-256-GCM(clair) || tag(16)`.

Le secret d'appairage ne passe jamais par le relais (il est dans le fragment). Sans lui, le relais ne peut ni se faire passer pour un téléphone (preuve invalide) ni s'interposer entre les deux (clés impossibles à dériver) : aucun code n'a besoin d'être comparé à l'écran.

## 5. Clair d'un paquet chiffré

```
seq(u64 big-endian) || op(u8) || payload
```

Chaque côté numérote ses paquets à partir de 1. Le récepteur ignore tout `seq` inférieur ou égal au dernier reçu (rejeu, réordonnancement).

### Acquittement et renvoi (téléphone -> ordinateur)

Le téléphone renvoie ses POST jusqu'à obtenir un `204`, mais un paquet peut encore se perdre entre le relais et l'ordinateur (réponse de long-poll coupée en route) ou être retenu par un relais malveillant. D'où :

- L'ordinateur n'accepte que des paquets contigus (`seq == dernier + 1`). Un paquet qui suit un trou est ignoré : un enregistrement n'est donc jamais transcrit amputé (un « pas » manquant inverse le sens d'une phrase).
- Il envoie ACK(dernier `seq` reçu dans l'ordre) tous les 16 paquets, à chaque END, et au premier paquet ignoré pour un trou.
- Le téléphone garde chaque paquet envoyé jusqu'à son acquittement. Si le plus ancien n'est pas acquitté au bout de 4 s (et que sa propre file d'envoi est vide), il renvoie tous ceux qui restent, dans l'ordre. Au-delà de 8 Mio en attente (environ 4 minutes d'audio), il les abandonne.
- Seul un START peut sauter un trou : il abandonne l'enregistrement inachevé. Le téléphone peut ainsi toujours repartir, y compris après un rechargement qui fait sauter sa numérotation (ci-dessous).

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
| `0x14` ACK | ordinateur -> téléphone | `seq(u64 BE)` | Dernier paquet du téléphone reçu sans trou avant lui (voir ci-dessus) |

## 6. Messages texte (non chiffrés)

Seuls deux messages circulent en clair, avant qu'une clé existe :

- téléphone -> ordinateur, dans la boîte de l'ordinateur du salon d'accueil :

  ```json
  {"type": "hello", "key": "<clé publique du téléphone>", "to": "<clé publique de l'ordinateur>", "inbox": "<tag>", "proof": "<preuve>"}
  ```

  Les clés sont ici en base64 standard (65 octets, préfixe `0x04`). `to` indique quel QR code le téléphone a scanné. `proof` est la sortie HKDF de label `hello-proof` (32 octets, base64url) : l'ordinateur la compare en temps constant et ignore en silence un `hello` dont la preuve est fausse (la clé du QR code est tout de même consommée).
- ordinateur -> téléphone :

  ```json
  {"type": "stale", "to": "<clé publique du téléphone>"}
  ```

  L'ordinateur n'a pas (ou plus) la clé privée correspondant à `to` : le QR code scanné est périmé. Seul le détenteur du secret de l'ordinateur peut écrire dans une boîte de téléphone, un tiers ne peut donc pas forger ce message.

## 7. Déroulement

1. L'ordinateur crée le salon d'accueil (`POST /api/rooms`), génère une paire de clés et affiche le QR code.
2. Le téléphone ouvre le lien, génère sa paire de clés et son `tag`, envoie `hello` avec sa preuve, puis lit sa boîte `/phone/{tag}`. Il invite à confirmer sur l'ordinateur.
3. L'ordinateur reçoit le `hello` et vérifie la preuve : le téléphone devient **candidat** (60 s pour confirmer). La clé du QR code étant consommée, une nouvelle paire est générée et le QR code change ; un `hello` adressé à l'ancienne clé reçoit `stale`.
4. L'utilisateur confirme sur l'ordinateur (un clic : chaque nouveau téléphone doit être accepté). L'ordinateur ouvre un salon privé et envoie, dans la boîte du téléphone du salon d'accueil, un BOUND chiffré qui le désigne (renvoyé toutes les 2 s, 5 fois, au cas où un vieil onglet lisant la même boîte avalerait le premier). En cas de refus ou d'expiration : DENIED.
5. Le téléphone se déplace dans le salon privé, où il écrit dans `/desktop` et lit `/phone/{tag}` avec le secret reçu. L'ordinateur remplace alors le salon d'accueil : la photo d'un ancien QR code n'ouvre plus rien, et quiconque l'a photographié ne peut au plus qu'encombrer le salon d'accueil (8 boîtes de téléphone), jamais le téléphone appairé.
6. Chaque enregistrement : START, AUDIO..., END (ou CANCEL). L'ordinateur acquitte (ACK), répond RECEIVED puis transcrit et colle le texte.
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
- Une transcription prête plus de 20 s après la réception de END n'est pas collée (l'utilisateur est probablement passé à une autre fenêtre) : l'interface signale seulement qu'une transcription a été écartée.
- Un enregistrement est limité à 20 minutes d'audio.

## 10. Modèle de menace

Le relais ne voit que des octets opaques et des identifiants aléatoires ; il garde tout en RAM et ne journalise ni IP ni identifiant de salon. Grâce au secret d'appairage, le chiffrement tient face à l'opérateur du relais et au réseau sans comparaison de code, tant que la page du téléphone est honnête : c'est le même serveur qui sert la page et son code cryptographique, un relais compromis pourrait donc servir une page modifiée.

### Vérification de la page servie

Pour limiter ce risque, Plumed embarque à la compilation une copie exacte des fichiers de la page (`protocol/src/page.rs`, la même table que celle que sert le relais). Une seule fois par session, entre 2 et 9 s (au hasard) après l'affichage du QR code, Plumed télécharge chaque fichier de la page comme le ferait un téléphone (nouvelle connexion, User-Agent et en-têtes d'un navigateur mobile, sans secret de salon) et le compare octet par octet à sa copie. À la moindre différence, PhoneMic s'arrête avec un message d'erreur. La vérification n'est jamais périodique : elle coûte au relais un chargement de page par session.

Pour que cette vérification ne confonde pas un relais simplement plus récent avec un relais compromis, la page est liée à la version du protocole : toute modification de la page impose d'incrémenter `PROTOCOL_VERSION` (un test de `protocol/src/page.rs` épingle une somme de contrôle de la page avec la version et échoue sinon). La version étant vérifiée avant la page, il y a donc trois cas :

- relais sur une version plus récente : Plumed n'est pas à jour, il le dit et propose de se mettre à jour, même si la vérification automatique des mises à jour est désactivée ;
- relais sur une version plus ancienne : le relais n'est pas à jour, Plumed le dit ;
- même version mais page différente : le relais est peut-être compromis, PhoneMic s'arrête.

Le relais et Plumed doivent donc être publiés ensemble à chaque changement de la page ou du protocole.

Limites honnêtes : cette vérification décourage une modification de masse de la page (un relais qui la modifierait pour tout le monde serait détecté par chaque Plumed), mais pas une attaque ciblée. Un relais malveillant peut distinguer la requête de Plumed de celle d'un vrai téléphone : empreinte TLS de `reqwest` différente de celle d'un navigateur, adresse IP de l'ordinateur déjà vue lors de la création du salon, absence des requêtes annexes d'un navigateur. Il pourrait alors servir la page honnête à Plumed et une page modifiée au téléphone.

### Héberger son propre relais

La vraie garantie reste donc la confiance dans le relais. Chacun est libre d'utiliser son propre relais : le code est public, le serveur tient dans un seul binaire (ou une image Docker) sans base de données ni stockage disque, et l'adresse se change dans les réglages de PhoneMic. Il peut tourner sur le serveur d'un établissement, derrière son pare-feu, ou sur une machine aussi sécurisée que nécessaire (serveur durci, hébergement certifié HDS, réseau interne uniquement accessible en VPN). Seule contrainte : le relais doit être joignable en `https://` par le téléphone, car un navigateur n'autorise ni le micro ni WebCrypto hors contexte sécurisé ; Plumed refuse d'ailleurs toute adresse `http://` autre que `localhost`.
