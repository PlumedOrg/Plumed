---
title: Dictée
description: Dicter dans n'importe quelle application avec un raccourci clavier.
sidebar:
  order: 1
---

Plumed écrit le texte dicté là où se trouve le curseur, dans n'importe quelle application. Les raccourcis se changent dans **Paramètres > Raccourcis**.

| Action | Raccourci par défaut |
| --- | --- |
| Dicter | `Ctrl+Espace` |
| Coller la dernière transcription | `Ctrl+Maj+Espace` |
| Annuler l'enregistrement en cours | `Ctrl+Retour arrière` |

## Mode d'enregistrement

Dans **Paramètres > Général > Mode d'enregistrement** :

- **Appuyer pour parler** (par défaut) : maintenez le raccourci pendant que vous parlez, relâchez pour transcrire.
- **Appuyer pour démarrer/arrêter** : un appui démarre l'enregistrement, un second l'arrête.

Pendant l'enregistrement, un indicateur affiche le niveau du micro et un aperçu du texte (**Paramètres > Indicateur** pour sa taille, sa position et quand l'afficher).

## Insertion du texte

**Paramètres > Général > Mode d'insertion de texte** choisit comment le texte arrive dans l'application : collage standard (`Ctrl+V`), collage pour terminal (`Ctrl+Maj+V`) ou saisie directe. Si le texte ne s'affiche pas, il est de toute façon dans le presse-papiers : appuyez sur `Ctrl+V`.

## Sous Wayland

Sous Wayland, une application ne peut pas écouter le clavier globalement : les raccourcis se créent dans les paramètres de votre bureau, qui lancent une commande Plumed. L'application affiche les étapes pour votre bureau (GNOME, KDE, ...). Le mode « Appuyer pour démarrer/arrêter » est alors le seul possible.

```sh
plumed --transcription   # démarre ou arrête une dictée
plumed --paste-last      # colle la dernière transcription
plumed --cancel          # annule l'enregistrement en cours
```

La saisie directe du texte demande l'accès à `/dev/uinput`, par une règle udev que Plumed propose d'installer.

## Historique

La page d'accueil garde les dernières transcriptions. La flèche d'une transcription montre le texte complet et ce que chaque étape des [corrections](/docs/fonctionnalites/corrections/) a changé.
