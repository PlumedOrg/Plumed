---
title: Installation
description: Télécharger et installer Plumed sous Windows, Linux et macOS.
---

Sur cette page : [Windows](#windows) · [Linux](#linux) · [macOS](#macos) · [Premier lancement](#premier-lancement)

Téléchargement direct de la dernière version stable :

| Système | Fichier |
| --- | --- |
| Windows (x64) | [Plumed_x64-setup.exe](https://github.com/PlumedOrg/Plumed/releases/latest/download/Plumed_x64-setup.exe) |
| Linux Debian, Ubuntu | [Plumed_amd64.deb](https://github.com/PlumedOrg/Plumed/releases/latest/download/Plumed_amd64.deb) |
| Linux Fedora, openSUSE | [Plumed_amd64.rpm](https://github.com/PlumedOrg/Plumed/releases/latest/download/Plumed_amd64.rpm) |
| Linux Arch | [Plumed_amd64.pkg.tar.zst](https://github.com/PlumedOrg/Plumed/releases/latest/download/Plumed_amd64.pkg.tar.zst) |
| Linux (autres distributions) | [Plumed_amd64.AppImage](https://github.com/PlumedOrg/Plumed/releases/latest/download/Plumed_amd64.AppImage) |
| macOS Apple Silicon (M1 et suivants) | [Plumed_aarch64_darwin.dmg](https://github.com/PlumedOrg/Plumed/releases/latest/download/Plumed_aarch64_darwin.dmg) |
| macOS Intel | [Plumed_x86_64_darwin.dmg](https://github.com/PlumedOrg/Plumed/releases/latest/download/Plumed_x86_64_darwin.dmg) |

Les versions bêta sont sur la [page des releases](https://github.com/PlumedOrg/Plumed/releases), et Plumed peut aussi les proposer lui-même (canal bêta dans **Paramètres > En ligne**).

## Windows

L'installateur s'installe sans droits d'administrateur. Il n'est pas signé : Windows SmartScreen affiche un avertissement, il faut choisir « Informations complémentaires » puis « Exécuter quand même ».

## Linux

Sous Wayland, la saisie directe du texte demande une règle udev que l'application propose d'installer, et les raccourcis clavier se configurent dans votre système (voir [Dictée](/docs/fonctionnalites/dictee/#sous-wayland)).

## macOS

macOS 13.4 ou plus récent. L'application est signée ad hoc mais pas notarisée (pas de compte développeur Apple), macOS la bloque donc au premier lancement. Autorisez-la dans **Réglages Système > Confidentialité et sécurité** (« Ouvrir quand même »), ou exécutez dans un terminal :

```sh
xattr -dr com.apple.quarantine /Applications/Plumed.app
```

## Premier lancement

Un assistant pose quelques questions, une par étape :

1. **Le modèle de reconnaissance vocale** à télécharger (une seule fois, depuis Hugging Face ou à défaut depuis le relais de Plumed `models.plumed.fr`). Ensuite, Plumed fonctionne entièrement hors ligne.
2. **Les statistiques d'utilisation anonymes** : désactivées sauf si vous répondez oui.
3. **La vérification des mises à jour des règles médicales**.
4. **Le canal de mise à jour** (stable ou bêta).

Toutes ces réponses se changent ensuite dans **Paramètres > En ligne**. Le **Tutoriel**, dans la barre latérale, fait ensuite essayer les commandes de dictée une par une.
