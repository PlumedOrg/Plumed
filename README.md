<p align="center"><img src="src-tauri/icons/128x128@2x.png" alt="Icône de Plumed" width="128"></p>

# Plumed

Dictée vocale médicale en français, gratuite, sans compte et hors ligne, dans n'importe quelle application. Réalisée par [Olivier Cornelis](https://olicorne.org/fr), interne en psychiatrie.

**Présentation et documentation : [plumed.fr](https://plumed.fr)**

Plumed n'est cependant pas (encore ?) un dispositif médical certifié : n'y utilisez pas d'information identifiante (noms de patients, ...).

<p align="center"><img src="docs/screenshots/accueil.png" alt="Écran d'accueil de Plumed : statistiques, visualisation du micro et dernière dictée" width="420"></p>

## Installation

Téléchargement direct de la dernière version stable :

| Système                              | Fichier                                                                                                             |
| ------------------------------------ | ------------------------------------------------------------------------------------------------------------------- |
| Windows (x64)                        | [Plumed_x64-setup.exe](https://github.com/PlumedOrg/Plumed/releases/latest/download/Plumed_x64-setup.exe)           |
| Linux Debian, Ubuntu                 | [Plumed_amd64.deb](https://github.com/PlumedOrg/Plumed/releases/latest/download/Plumed_amd64.deb)                   |
| Linux Fedora, openSUSE               | [Plumed_amd64.rpm](https://github.com/PlumedOrg/Plumed/releases/latest/download/Plumed_amd64.rpm)                   |
| Linux Arch                           | [Plumed_amd64.pkg.tar.zst](https://github.com/PlumedOrg/Plumed/releases/latest/download/Plumed_amd64.pkg.tar.zst)   |
| Linux (autres distributions)         | [Plumed_amd64.AppImage](https://github.com/PlumedOrg/Plumed/releases/latest/download/Plumed_amd64.AppImage)         |
| macOS Apple Silicon (M1 et suivants) | [Plumed_aarch64_darwin.dmg](https://github.com/PlumedOrg/Plumed/releases/latest/download/Plumed_aarch64_darwin.dmg) |
| macOS Intel                          | [Plumed_x86_64_darwin.dmg](https://github.com/PlumedOrg/Plumed/releases/latest/download/Plumed_x86_64_darwin.dmg)   |

Versions bêta, avertissements au premier lancement, Wayland : voir le [guide d'installation](https://plumed.fr/docs/installation/).

## Documentation

[Dictée](https://plumed.fr/docs/fonctionnalites/dictee/) · [Corrections](https://plumed.fr/docs/fonctionnalites/corrections/) · [Médicaments](https://plumed.fr/docs/fonctionnalites/medicaments/) · [Textes types](https://plumed.fr/docs/fonctionnalites/textes-types/) · [PhoneMic](https://plumed.fr/docs/fonctionnalites/phonemic/) · [API locale](https://plumed.fr/docs/fonctionnalites/api-locale/) · [FAQ](https://plumed.fr/docs/faq/) · [Dépannage](https://plumed.fr/docs/depannage/) · [Confidentialité](PRIVACY_POLICY.md)

## Remerciements et licences

Plumed est un logiciel libre sous licence [GNU AGPL v3](LICENSE). C'est initialement un fork de **[Murmure](https://www.murmure.app/)** par [Al1x-ai](https://www.al1x-ai.com/) ([code source](https://github.com/Kieirra/murmure)) : soutenez le projet original ! Il reprend le mode médical de [ParakeetWeb](https://github.com/thiswillbeyourgithub/parakeet_web) et les modèles [UltiMed](https://github.com/UltiMed-Org), du même auteur.

Tous les remerciements et licences (UltiMed, LPSN, Grammalecte, ...) : [plumed.fr/docs/licences](https://plumed.fr/docs/licences/), ainsi que [src-tauri/medical-data/SOURCE.md](src-tauri/medical-data/SOURCE.md) et [src-tauri/grammar-data/SOURCE.md](src-tauri/grammar-data/SOURCE.md).
