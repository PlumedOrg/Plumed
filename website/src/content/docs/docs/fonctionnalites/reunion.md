---
title: Réunion
description: Transcrire une réunion jusqu'à 4 heures en distinguant les intervenants, sur l'ordinateur.
sidebar:
  order: 9
---

:::caution[Expérimental]
Cette fonctionnalité est une préversion, disponible à partir de Plumed 1.4 : attendez-vous à des bugs.
:::

La page **Réunion** (barre latérale) transcrit une réunion de 4 heures au plus et distingue jusqu'à 4 intervenants. Tout se passe sur l'ordinateur : l'audio n'est envoyé nulle part.

## Démarrer

1. À la première ouverture, cliquer sur **Télécharger** pour récupérer le modèle de distinction des intervenants (environ 106 Mo, une seule fois).
2. Choisir la source : **Microphone**, **PhoneMic** (le téléphone doit d'abord être connecté sur la page PhoneMic ; chaque enregistrement fait sur le téléphone s'ajoute à la réunion) ou **Fichier audio** (WAV, MP3, M4A, FLAC, OGG ; une barre montre l'avancement).
3. Cliquer sur **Démarrer la réunion**. Les lignes apparaissent environ 30 secondes après avoir été prononcées, le temps de reconnaître l'intervenant.

**Arrêter** puis relancer continue la même réunion (après une coupure, par exemple). **Nouvelle réunion** efface la transcription (cliquer deux fois).

Pendant une réunion, l'ordinateur ne se met pas en veille tout seul, et la dictée habituelle est refusée (l'indicateur l'affiche).

## Transcription

Chaque ligne a la forme `- Paul: bonjour à tous`. Les intervenants s'appellent A, B, C et D : leurs noms se modifient au-dessus de la transcription. Les horodatages sont facultatifs, depuis le début de la réunion ou à partir d'une heure de début (par exemple `14:00`), avec un décalage en secondes si besoin. Le bouton de copie copie toute la transcription.

Seules les corrections orthographiques médicales s'appliquent (si elles sont activées), pas les autres corrections de la dictée.

## Sauvegarde

Désactivée par défaut. Sur la page Réunion, on peut choisir de sauvegarder au fil de l'eau l'**audio**, la **transcription** et la **progression**, dans le dossier temporaire ou, avec **La garder dans le dossier de l'application**, dans le dossier de données de Plumed. Le réglage s'applique à la réunion suivante.

Si Plumed se ferme en pleine réunion (plantage, coupure de courant), il propose au démarrage de **Reprendre** la réunion ou de **Supprimer** la sauvegarde. Avec la transcription sauvegardée, la réunion reprend là où elle s'était arrêtée (les lettres des intervenants peuvent alors changer pour la suite) ; avec l'audio seul, il est transcrit à nouveau. **Nouvelle réunion** supprime la sauvegarde. Voir la [politique de confidentialité](/confidentialite/).
