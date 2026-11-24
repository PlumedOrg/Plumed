# Relais PhoneMic de Plumed

> **Expérimental :** PhoneMic (le microphone distant) est une fonctionnalité en préversion : attendez-vous à des bugs et à des changements de protocole incompatibles entre les versions de Plumed.

Ce répertoire contient tout ce qui tourne côté serveur pour Plumed : le relais du microphone distant, un petit relais HTTPS en Rust (axum) qui sert aussi la page du téléphone, et, en tant que second service indépendant, le [relais de modèles](#relais-de-modèles) qui diffuse les modèles de reconnaissance vocale depuis Hugging Face. Il a été écrit avec Claude Code.

Fonctionnement :

1. Plumed (sur l'ordinateur) crée un salon sur le relais et affiche un QR code : `https://turn.plumed.fr/#{room}.{phone secret}.{desktop public key}`. Le salon a un secret par côté, et le QR code ne porte que celui du téléphone : quiconque le photographie ne peut ni lire ce que les téléphones envoient à l'ordinateur, ni fermer le salon.
2. Le téléphone ouvre la page, génère sa propre clé et envoie un `hello` contenant sa clé publique et le nom aléatoire de sa propre boîte de réception (pour que les téléphones d'un même salon ne prennent jamais les trames des autres). Les deux écrans affichent le même code d'appairage (par exemple `524C-F8E8-D5A4-2D82`) et l'utilisateur de l'ordinateur le confirme. La confirmation (chiffrée) désigne un nouveau salon ouvert pour ce seul téléphone, qui s'y déplace. Le QR code passe alors lui aussi à un nouveau salon : la photo d'un ancien QR code n'ouvre donc rien, et quelqu'un qui détient le QR code courant peut au plus encombrer ses 8 boîtes de réception de téléphone pour retarder l'appairage suivant, sans jamais perturber le téléphone appairé.
3. À partir de là, chaque trame est chiffrée de bout en bout (ECDH P-256, HKDF-SHA256, AES-256-GCM), avec une clé par sens liée aux deux clés publiques : un paquet ne peut donc être ni renvoyé à son expéditeur ni rejoué dans un autre appairage. Les refus et les avis « un autre téléphone a pris la main » sont chiffrés eux aussi, le relais ne peut donc pas les falsifier. Le relais ne fait que transmettre des octets opaques entre boîtes de réception : il ne peut rien entendre, garde tout en RAM et ne journalise ni IP ni identifiant de salon. Cela tient face à l'opérateur du relais et au réseau tant que la page du téléphone elle-même est honnête : le même serveur sert la page et son code cryptographique, un relais compromis pourrait donc servir une page modifiée. Utilisez un relais auquel vous ou votre établissement faites confiance, ou hébergez le vôtre.

Le transport se fait en simple long-polling HTTPS (ni WebSocket, ni WebRTC, ni serveur TURN) : il fonctionne donc derrière tout proxy ou pare-feu qui laisse passer le HTTPS.

## Organisation

| Chemin | Contenu |
| --- | --- |
| `protocol/` | Crate Rust partagée avec Plumed (`src-tauri`) : encapsulation des enveloppes, rôles, texte clair `seq \|\| op \|\| payload`, et `test-vectors.json` |
| `server/src/` | Le relais (salons, limiteur de débit, API HTTP, page du téléphone embarquée) |
| `server/static/` | La page du téléphone (modules ES simples, sans étape de build) : français par défaut, bascule vers l'anglais |
| `server/static-tests/` | Tests Node du JS du téléphone (format filaire, crypto contre les vecteurs partagés, VAD) |
| `server/src/serve.rs` | Utilitaires de service partagés par les deux binaires (environnement, IP du client derrière le proxy, arrêt, `--healthcheck`) |
| `protocol_details.md` | Spécification détaillée du protocole de microphone distant de ParakeetWeb (transport par relais uniquement, échange de clés, encapsulation) dont le protocole de Plumed est dérivé |
| `models/` | Le relais de modèles `plumed-models` (voir [Relais de modèles](#relais-de-modèles)) |
| `Dockerfile`, `docker-compose.yml`, `env.example` | Déploiement des deux services (cibles du Dockerfile : `relay` et `models`) |

Les fichiers `wire.js` et `crypto.js` du téléphone reflètent le code Rust (`protocol/src/lib.rs`, `src-tauri/src/remote_mic/crypto.rs`). Cette duplication entre langages est inévitable, les deux côtés sont donc verrouillés par `protocol/test-vectors.json` : modifiez-les ensemble.

## Fonctionnalités de la page du téléphone

- Mode manuel : touchez pour démarrer et arrêter, ou maintenez le bouton appuyé pour parler (push-to-talk).
- Mode mains libres : le téléphone écoute en continu et n'envoie que la parole. Une VAD à base d'énergie avec un plancher de bruit adaptatif (trames de 20 ms) ouvre un énoncé après 60 ms de parole, inclut les 400 ms entendues juste avant, le ferme après 900 ms de silence, et l'abandonne s'il contenait moins de 250 ms de parole. Le silence ne quitte jamais le téléphone.
- Pause et reprise, envoi d'un fichier audio (20 minutes maximum).
- Verrou de mise en veille de l'écran pendant la connexion, pour que le téléphone ne s'endorme pas.
- Reconnexion automatique après une perte de réseau. Le secret du salon est retiré de la barre d'adresse dès qu'il est lu. Une fois appairé, le téléphone conserve sa session dans IndexedDB (clés AES non extractibles, salon privé, compteurs de séquence) pendant 12 h, la durée de vie de l'appairage côté ordinateur : un rechargement de la page, ou le téléphone qui tue l'onglet en arrière-plan, reprend la session sans nouveau scan. L'enregistrement est supprimé à toute erreur définitive (salon fermé, téléphone remplacé, appairage refusé) et lorsqu'un nouveau QR code est scanné.

## API HTTP

| Requête | Réponse |
| --- | --- |
| `POST /api/rooms` | `201 {"room","desktop","phone"}` (un secret par côté), `403` (envoyée par un navigateur : seule l'application de bureau crée des salons), `429` (limites par IP) ou `503` (serveur plein ou occupé) |
| `POST /api/rooms/{room}/desktop` + secret du téléphone | Ajoute une enveloppe à la boîte de réception de l'ordinateur : `204`, `400` (mal formée), `429` (boîte pleine : 1 Mio, ou 64 Mio pour tout le relais) |
| `GET /api/rooms/{room}/desktop` + secret de l'ordinateur | Long-poll (25 s) : `200` avec les enveloppes en attente, `204` si rien n'est arrivé, `404` une fois le salon fermé |
| `POST /api/rooms/{room}/phone/{tag}` + secret de l'ordinateur | Ajoute une enveloppe à la boîte de réception de ce téléphone (`tag` : 16 à 43 caractères base64url choisis par le téléphone) |
| `GET /api/rooms/{room}/phone/{tag}` + secret du téléphone | Long-poll de la boîte de réception de ce téléphone. Au plus 8 boîtes de téléphone par salon : celles inactives depuis une minute libèrent la place, sinon `429` |
| `DELETE /api/rooms/{room}` + secret de l'ordinateur | `204` |
| `GET /healthz` | `ok` |

Les secrets passent dans l'en-tête `x-room-secret`. Un salon inconnu et un mauvais secret (y compris celui de l'autre côté) répondent tous deux `404`. Les salons sont supprimés après 10 minutes sans polling ou 24 heures d'existence.

Toute requête peut aussi recevoir `429` lorsque son client a déjà 4 requêtes en cours par salon qu'il peut ouvrir (400 avec la valeur par défaut de `PLUMED_RELAY_ROOMS_PER_IP`), ou `503` lorsque le relais est occupé (2048 requêtes en cours, ou 32 Mio de corps en cours de lecture). Un corps doit arriver en moins de 10 s (`408`) et déclarer au plus 1 Mio (`413`).

## Configuration

| Variable | Défaut | Signification |
| --- | --- | --- |
| `PLUMED_RELAY_BIND` | `0.0.0.0:5174` | Adresse d'écoute (définie par `docker-compose.yml`) |
| `PLUMED_RELAY_TRUST_PROXY` | `false` | Prendre l'IP du client dans la dernière entrée de `X-Forwarded-For`, uniquement quand la requête vient d'une adresse de loopback ou privée (le reverse proxy). `docker-compose.yml` la met à `true` |
| `PLUMED_RELAY_MAX_ROOMS` | `500` | Salons ouverts simultanément |
| `PLUMED_RELAY_ROOMS_PER_IP` | `100` | Salons ouverts simultanément pour une IP cliente (IPv6 : par /64), ce qui fixe aussi sa part de requêtes en cours (4 par salon). Un hôpital derrière une seule adresse NAT la partage |
| `PLUMED_RELAY_ROOMS_PER_IP_PER_HOUR` | `300` | Créations de salon par IP cliente et par heure |

Les trois dernières peuvent être définies dans `.env` (copiez `env.example`), `docker-compose.yml` les transmet au conteneur.

## Relais de modèles

Plumed n'embarque pas son modèle de reconnaissance vocale : au premier lancement, l'application télécharge depuis Hugging Face le modèle choisi par l'utilisateur. Les réseaux hospitaliers bloquent souvent huggingface.co mais laissent passer plumed.fr, l'application se rabat donc sur `plumed-models`, un second service indépendant de ce répertoire : un portage en Rust du relais HuggingFace de ParakeetWeb (`signaling/hf-relay.js`), écrit avec Claude Code.

- `GET` / `HEAD /models/<owner>/<name>/<path>?revision=<commit>` renvoie le fichier de ce dépôt à ce commit (le schéma se trouve dans `protocol/src/models.rs`, `relay_path`, utilisé aussi par l'application). `GET /healthz` répond `ok`.
- Il diffuse depuis huggingface.co sans écrire sur disque et sans mise en tampon au-delà de la fenêtre de contre-pression du flux (mémoire constante, le coût n'est que de la bande passante). `Range`, `If-Range` et `If-None-Match` sont transmis ; `Content-Length`, `Content-Range`, `Accept-Ranges` et `ETag` reviennent, l'application peut donc reprendre un téléchargement interrompu. `HEAD` s'arrête au hub (`X-Linked-Size`, `X-Linked-Etag`) et ne touche jamais le CDN.
- Jamais un proxy ouvert : seules les paires `repo@revision` de la liste autorisée sont servies, tout le reste est un `404` sans contacter le hub ; chaque segment de chemin doit correspondre à `[A-Za-z0-9._-]+` (pas de `..`, pas d'encodage pourcent) ; les redirections sont suivies à la main, 5 au plus, et chaque saut doit être en https sur `huggingface.co`, `*.huggingface.co` ou `*.hf.co`, sinon `502`.
- Le `HF_TOKEN` optionnel n'est envoyé qu'à huggingface.co, jamais à son CDN.
- Les flux de fichiers simultanés sont plafonnés globalement et par IP cliente (`503` avec `Retry-After: 5`). Les en-têtes amont doivent arriver en moins de 30 s (`504`), un corps qui n'envoie rien pendant 60 s est coupé. Les IP clientes ne sont jamais journalisées ; les erreurs ne journalisent que le dépôt et le fichier.

La liste autorisée vaut par défaut `MODEL_REPOS` dans `protocol/src/models.rs`, la liste même sur laquelle est construit le catalogue de l'application (une seule source de vérité) : pour proposer un nouveau modèle ou une nouvelle révision, mettez à jour cette liste et le catalogue (`src-tauri/src/model/catalog.rs`), puis redéployez. Gardez l'ancienne révision dans la liste tant que des versions publiées de l'application la téléchargent encore.

| Variable | Défaut | Signification |
| --- | --- | --- |
| `PLUMED_MODELS_BIND` | `0.0.0.0:5175` | Adresse d'écoute (définie par `docker-compose.yml`) |
| `PLUMED_MODELS_TRUST_PROXY` | `false` | Identique à `PLUMED_RELAY_TRUST_PROXY`, `docker-compose.yml` la met à `true` |
| `PLUMED_MODELS_REPOS` | les dépôts de l'application | Paires `owner/name@revision` servies, séparées par des virgules ; une entrée mal formée empêche le démarrage |
| `PLUMED_MODELS_MAX_STREAMS` | `32` | Flux de fichiers simultanés |
| `PLUMED_MODELS_STREAMS_PER_IP` | `8` | Flux de fichiers simultanés par IP cliente (IPv6 : par /64) |
| `HF_TOKEN` | vide | Jeton Hugging Face, nécessaire uniquement si le hub limite le débit du serveur |

Toutes sauf l'adresse d'écoute peuvent être définies dans `.env`, `docker-compose.yml` les transmet au conteneur.

### Versions de l'application

Le même service relaie aussi les versions publiées de l'application, pour que la mise à jour automatique fonctionne là où github.com est bloqué (`models/src/releases.rs`, schéma dans `protocol/src/releases.rs`, partagé avec l'application ; écrit avec Claude Code). L'application interroge d'abord GitHub et n'utilise ces routes qu'en cas d'échec :

- `GET /releases/latest/download/latest.json` : le manifeste de mise à jour de la dernière version stable, depuis `https://github.com/PlumedOrg/Plumed/releases/latest/download/latest.json`.
- `GET /releases/download/<tag>/<asset>` : un fichier d'une version, depuis `https://github.com/PlumedOrg/Plumed/releases/download/<tag>/<asset>`. L'application réécrit vers cette route l'URL de l'installeur trouvée dans `latest.json` lorsqu'elle s'est rabattue sur le relais.
- `GET /releases/list` : les 10 versions les plus récentes (`https://api.github.com/repos/PlumedOrg/Plumed/releases?per_page=10`), lues par le canal bêta. Gardées en mémoire 5 minutes (au plus 12 appels à l'API GitHub par heure, la limite sans authentification étant de 60), une récupération à la fois, 4 Mio au plus.

Même durcissement que pour les routes de modèles : seul le dépôt `PlumedOrg/Plumed` (il ne fait pas partie du chemin, aucun autre ne peut donc être demandé), `<tag>` et `<asset>` doivent être des jetons `[A-Za-z0-9._-]+`, pas de query string, tout le reste est un `404` sans contacter GitHub. Les redirections sont suivies à la main, 5 au plus, chaque saut en https sur `github.com`, `objects.githubusercontent.com` ou `release-assets.githubusercontent.com` (sinon `502`). Les fichiers sont diffusés sans mise en tampon, `Range` / `If-Range` / `If-None-Match` sont transmis, et les flux de versions partagent les plafonds globaux et par IP des flux de modèles (la liste prend elle aussi un emplacement par IP). Le `HF_TOKEN` n'est jamais envoyé à GitHub.

Un relais ne peut pas falsifier une mise à jour : l'application n'installe qu'un installeur dont la signature minisign correspond à la clé publique de mise à jour compilée dans l'application (`plugins.updater.pubkey` dans `src-tauri/tauri.conf.json`), et la clé privée ne quitte jamais la machine du mainteneur et les secrets GitHub. Un relais compromis pourrait au plus retenir les mises à jour ou servir une version signée plus ancienne, que l'application refuse d'installer puisqu'elle ne revient jamais en arrière.

## Développement

```sh
cargo test                                  # protocol + relay (build output goes to ../plumed_build_ignore_backups/)
node --test server/static-tests/*.test.mjs  # phone page
cargo run -p plumed-relay                   # http://localhost:5174
cargo run -p plumed-models                  # http://localhost:5175 (model relay)
```

`cargo test` exécute aussi le relais de modèles contre un faux Hugging Face (un hub qui redirige vers un CDN) et un faux GitHub, sur de vrais sockets (`models/tests/relay.rs`).

Les navigateurs ne donnent accès au microphone qu'aux origines sécurisées : tester sur un vrai téléphone nécessite donc HTTPS (par exemple un reverse proxy avec un certificat).

## Déploiement

Le relais tourne derrière un reverse proxy qui termine le TLS. Avec Caddy :

```
turn.plumed.fr {
    reverse_proxy 127.0.0.1:5174
}
```

Le relais de modèles a besoin de son propre bloc. `flush_interval -1` envoie chaque morceau dès son arrivée (pas de mise en tampon d'un fichier de 650 Mo), et il ne doit y avoir aucune directive `encode` dessus : la compression supprimerait `Content-Length` et casserait la reprise par `Range` :

```
models.plumed.fr {
    reverse_proxy 127.0.0.1:5175 {
        flush_interval -1
    }
}
```

Laissez le journal d'accès de Caddy désactivé sur les deux (il l'est tant qu'aucune directive `log` n'est définie) : il enregistrerait les IP clientes et les identifiants de salon ou les fichiers téléchargés, que les relais eux-mêmes ne journalisent jamais.

Ensuite, sur le serveur, dans une copie de ce répertoire : `cp env.example .env && sudo docker compose up -d --build`. Les deux conteneurs sont en lecture seule, non privilégiés (distroless, non-root, sans capabilities) et vérifient leur propre santé avec `plumed-relay --healthcheck` et `plumed-models --healthcheck`.

Les mainteneurs déploient avec un `deploy.sh` local, ignoré par git, qui synchronise ce répertoire vers le VPS par rsync, construit les deux images, remplace les deux conteneurs et vérifie `https://turn.plumed.fr/healthz` et `https://models.plumed.fr/healthz`.
