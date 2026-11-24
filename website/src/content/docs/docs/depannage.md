---
title: Dépannage
description: Que faire quand la dictée ne marche pas comme prévu.
---

## Le texte n'apparaît pas

La transcription est toujours copiée dans le presse-papiers : appuyez sur `Ctrl+V` pour la coller. Si cela arrive souvent, essayez un autre **Mode d'insertion de texte** dans **Paramètres > Général** (le collage pour terminal convient aux terminaux, mais peut gêner certains logiciels comme LibreOffice). Le raccourci **Coller la dernière transcription** (`Ctrl+Maj+Espace`) recolle la dernière dictée.

## Le raccourci ne fait rien

- Un autre logiciel utilise peut-être déjà ce raccourci : changez-le dans **Paramètres > Raccourcis**.
- Sous Wayland (Linux), les raccourcis doivent être créés dans les paramètres de votre bureau : voir [Sous Wayland](/docs/fonctionnalites/dictee/#sous-wayland).

## Le microphone n'est pas détecté

Vérifiez le micro choisi dans **Paramètres > Général > Microphone** et qu'il est bien branché. Sous macOS, autorisez Plumed dans **Réglages Système > Confidentialité et sécurité > Microphone**.

## Windows ou macOS bloque l'installation

Plumed n'est pas signé par un certificat payant. Voir les instructions de [l'installation](/docs/installation/) pour Windows (SmartScreen) et macOS (Gatekeeper).

## La reconnaissance se trompe

- Vérifiez qu'un modèle UltiMed est sélectionné dans **Paramètres > Modèle**.
- Parlez distinctement, micro proche, sans bruit de fond.
- Ajoutez les mots mal reconnus au dictionnaire personnel ou en règle de correction : voir la [FAQ](/docs/faq/#plumed-reconnaît-mal-un-mot-que-jutilise-souvent-que-faire-).
- La flèche d'une transcription sur la page d'accueil montre ce que chaque étape des corrections a changé, pour repérer une règle qui corrige à tort.

## Signaler un problème

Ouvrez un ticket sur [GitHub](https://github.com/PlumedOrg/Plumed/issues/new/choose). Joignez si possible les journaux de l'application : **Paramètres > Avancé > Niveau de log**, puis le bouton qui ouvre le dossier des logs.

:::caution
Aux niveaux **Debug** et **Trace**, les journaux peuvent contenir le texte de vos dictées. Relisez-les avant de les partager, et revenez ensuite au niveau par défaut.
:::
