---
title: API de transcription locale
description: Une API compatible OpenAI, sur cet ordinateur seulement, pour transcrire depuis vos propres programmes.
sidebar:
  order: 7
---

Désactivée par défaut : **Paramètres > Avancé > API de transcription locale**.

Une fois activée, Plumed sert sur `http://127.0.0.1:4800/v1` (port modifiable) une API compatible avec celle d'OpenAI : un programme ou un script qui sait parler à `POST /v1/audio/transcriptions` d'OpenAI fonctionne en changeant seulement son adresse de base. Le texte est transcrit exactement comme une dictée (vocabulaire médical, dictionnaire personnel, règles de correction et de formatage, grammaire, textes types), mais il n'est ajouté ni à l'historique ni aux statistiques de la page d'accueil.

```sh
curl http://127.0.0.1:4800/v1/audio/transcriptions \
  -H "Authorization: Bearer <CLÉ>" \
  -F file=@dictee.wav
# {"text":"..."}
```

Avec la bibliothèque Python d'OpenAI :

```python
from openai import OpenAI

client = OpenAI(base_url="http://127.0.0.1:4800/v1", api_key="<CLÉ>")
with open("dictee.wav", "rb") as f:
    print(client.audio.transcriptions.create(model="plumed", file=f).text)
```

## Format

Fichiers WAV PCM 16 bits uniquement (mono ou stéréo, toute fréquence d'échantillonnage), 100 Mo au plus. `response_format` vaut `json` (par défaut, `{"text": "..."}`) ou `text` ; `model`, `language` et `prompt` sont acceptés et ignorés. `GET /v1/models` liste un seul modèle, `plumed`.

## Sécurité

- L'API n'écoute que sur cet ordinateur (127.0.0.1), jamais sur le réseau.
- Chaque requête doit porter la clé affichée dans le réglage (`Authorization: Bearer <CLÉ>`), générée au hasard et régénérable.
- Les requêtes venant d'une page web (en-tête `Origin`) ou adressées à un autre nom que `localhost` ou `127.0.0.1` sont refusées, pour qu'un site ouvert dans le navigateur ne puisse pas s'en servir.
- Une seule transcription à la fois.

Ces réglages, la clé comprise, ne font jamais partie de l'export des paramètres.

## Sans fenêtre

`plumed serve [--port 4800]` sert la même API sans ouvrir l'application, jusqu'à Ctrl-C. Il refuse de démarrer tant que le réglage est désactivé, et lit la clé au démarrage (le relancer après avoir régénéré la clé). L'application et `plumed serve` ne peuvent pas utiliser le même port en même temps.
