---
title: Connexions réseau
description: Ce qui passe par le réseau, et comment tout couper.
sidebar:
  order: 8
---

La reconnaissance se fait sur votre ordinateur : l'audio et les transcriptions ne sont jamais envoyés sur internet. Une fois le modèle de reconnaissance vocale téléchargé, Plumed fonctionne entièrement hors ligne : aucune des connexions ci-dessous n'est nécessaire pour dicter.

**Paramètres > En ligne** liste tout ce qui passe par le réseau :

- **Mises à jour de l'application** : vérifiées à chaque démarrage puis une fois par jour si Plumed reste ouvert, sur le canal stable ou bêta, désactivable. Rien n'est installé sans votre clic.
- **Modèle de reconnaissance vocale** : téléchargé une seule fois, quand vous le choisissez, depuis Hugging Face ou à défaut depuis le relais Plumed `models.plumed.fr`. Il est conservé lors des mises à jour et se gère dans **Paramètres > Modèle**.
- **Mises à jour des règles médicales** : rien n'est téléchargé sans votre clic, et « Voir les changements » montre les lignes ajoutées et retirées avant d'installer.
- **Statistiques d'utilisation anonymes** : des totaux agrégés, jamais de texte ni d'audio. Désactivées sauf si vous avez accepté, réglables dans cet onglet.
- **[PhoneMic](/docs/fonctionnalites/phonemic/)** : seulement si vous l'activez.

Ces réglages ne font jamais partie de l'export des paramètres. L'[API de transcription locale](/docs/fonctionnalites/api-locale/), désactivée par défaut, n'écoute que sur l'ordinateur lui-même et ne passe pas par internet.

Le détail de chaque donnée est dans la [politique de confidentialité](/confidentialite/).
