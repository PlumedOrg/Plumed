---
title: Médicaments et termes médicaux
description: Les 45 000 règles qui corrigent les noms de médicaments et les termes médicaux mal reconnus.
sidebar:
  order: 3
---

Activée par défaut : **Paramètres > Corrections > Erreurs résiduelles**, interrupteur en haut de l'onglet.

Environ 45 000 règles préinstallées corrigent les noms de médicaments et les termes médicaux que la reconnaissance vocale entend mal :

| Transcription brute | Après correction |
| --- | --- |
| tacadol | Takadol |
| polllachiurie | pollakiurie |
| soucrural | sous-crural |

Ce sont les mêmes règles que le mode médical de [ParakeetWeb](http://parakeetweb.olicorne.org/?mode=med), appliquées en local. Provenance et licence (AGPL-3.0-or-later) : [`SOURCE.md`](https://github.com/PlumedOrg/Plumed/blob/medmain/src-tauri/medical-data/SOURCE.md).

## Règles de correction

Dans l'onglet **Erreurs résiduelles**, la section **Règles de correction** permet de :

- **Essayer les règles** : tapez une phrase pour voir ce que les règles en font.
- **Trouver une règle préinstallée** : cherchez par le mot correct (« Takadol ») ou par ce que la transcription a écrit (« tacadol »), puis modifiez le texte reconnu ou le remplacement, ou désactivez la règle.
- **Ajouter une règle** : un texte mal reconnu (expression régulière, sans distinction de casse, limites de mot ajoutées automatiquement) et son remplacement. Les règles ajoutées passent avant les règles préinstallées.
- **Vos modifications** : tout ce que vous avez changé, chaque règle pouvant être rétablie.

Ces modifications sont enregistrées séparément de vos règles de mise en forme : importer ou réinitialiser l'un ne touche pas l'autre. Pour corriger autre chose qu'un médicament (un nom propre, par exemple), utilisez plutôt les règles de **Formatage**.

## Mises à jour des règles

Si vous l'avez accepté au premier lancement (modifiable dans **Paramètres > En ligne**), Plumed vérifie au démarrage, puis une fois par jour s'il reste ouvert, s'il existe de nouvelles règles médicales. Rien n'est téléchargé sans votre clic, et « Voir les changements » montre les lignes ajoutées et retirées avant d'installer.
