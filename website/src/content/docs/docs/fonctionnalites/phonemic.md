---
title: PhoneMic
description: Dicter depuis son téléphone, chiffré de bout en bout, même hors du réseau de l'ordinateur.
sidebar:
  order: 6
---

:::caution[Expérimental]
Cette fonctionnalité est une préversion : attendez-vous à des bugs, et évitez d'y faire passer des contenus sensibles.
:::

Désactivé par défaut : **PhoneMic** dans la barre latérale.

Avec PhoneMic, le téléphone sert de micro, même s'il n'est pas sur le même réseau que l'ordinateur (4G, Wi-Fi invité d'un établissement) :

1. Cliquer sur **Connecter un téléphone** : un QR code s'affiche.
2. Le scanner avec l'appareil photo du téléphone. La page PhoneMic de Plumed s'ouvre (en français, ou en anglais), sans rien installer.
3. L'ordinateur demande « Un téléphone a scanné le QR code. Le connecter ? » : cliquer sur **Connecter**.
4. Dicter sur le téléphone : le texte transcrit est collé dans l'application active de l'ordinateur, comme pour une dictée normale. En mode mains libres, le téléphone écoute en continu et n'envoie que la parole, jamais les silences.

## Chiffrement

L'audio passe par un serveur relais (`https://turn.plumed.fr` par défaut, modifiable), mais il est chiffré de bout en bout entre le téléphone et Plumed : le relais ne peut pas l'écouter. Le QR code contient un secret d'appairage que le relais ne voit jamais : il ne peut donc ni se faire passer pour le téléphone ni s'intercaler.

Cette garantie suppose que le relais sert une page téléphone non modifiée (la page et son code de chiffrement viennent du relais). Plumed vérifie une fois par session que la page servie est identique à celle qu'il embarque, ce qui décourage une modification de masse mais pas une attaque ciblée : n'utilisez qu'un relais de confiance, ou hébergez le vôtre. Le relais voit en revanche les adresses IP et la taille des paquets chiffrés.

## Coupures et délais

- Le téléphone se reconnecte tout seul après une coupure réseau et renvoie les paquets perdus en route.
- Une fois appairé, il garde sa session 12 h : recharger la page ne demande pas de scanner à nouveau le QR code.
- Pour ne pas coller du texte dans une fenêtre que vous avez quittée entre-temps, une transcription prête plus de 20 secondes après la fin de l'enregistrement est seulement conservée dans l'historique, sans être collée, et un enregistrement en attente depuis plus de 30 secondes est abandonné.
- Une fois connecté, le bouton « Appairer un autre téléphone » réaffiche le QR code pour changer de téléphone.

Seul le relais HTTPS est utilisé (pas de WebRTC, souvent bloqué sur les réseaux d'établissement), et le proxy système est respecté.
