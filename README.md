<p align="center"><img src="src-tauri/icons/128x128@2x.png" alt="Icône de Plumed" width="128"></p>

# Plumed

Plumed est une application entièrement gratuite et open source de dictée vocale respectueuse de la vie privée : elle fonctionne localement sur votre ordinateur et permet de dicter dans **n'importe quelle application** (logiciel métier, traitement de texte, messagerie, etc.), avec une reconnaissance adaptée au **vocabulaire médical français** (médicaments, termes médicaux, comptes rendus, courriers). C'est un fork de [Murmure](https://github.com/Kieirra/murmure) qui reprend les modèles [UltiMed](https://github.com/UltiMed-Org) et les fonctionnalités du mode médical de [ParakeetWeb](http://parakeetweb.olicorne.org/?mode=med), la version navigateur. UltiMed et ParakeetWeb sont du même auteur que Plumed.

Plumed n'est pas un dispositif médical certifié : l'application est fournie telle quelle, sans garantie autre que sa [politique de confidentialité](PRIVACY_POLICY.md). Elle est distribuée sous licence [AGPL v3](LICENSE).

Plumed est réalisé par [Olivier Cornelis](https://olicorne.org/fr), interne en psychiatrie et développeur.

## Sommaire

- [Statut](#statut)
- [Installation](#installation)
- [Fonctionnalités](#fonctionnalités)
- [Captures d'écran](#captures-décran)
- [Confidentialité](#confidentialité)
- [Licence](#licence)
- [Remerciements et licences](#remerciements-et-licences)

## Statut

Version bêta, actuellement `0.2.0-beta.3`. Des bugs sont possibles. Les versions sont sur la [page des releases](https://github.com/PlumedOrg/Plumed/releases) ; la mise à jour automatique propose un canal stable et un canal bêta (**Paramètres > En ligne**).

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

Les versions bêta sont sur la [page des releases](https://github.com/PlumedOrg/Plumed/releases).

- **Windows** : l'installateur (NSIS) s'installe sans droits d'administrateur. Il n'est pas signé : Windows SmartScreen affiche un avertissement, il faut choisir « Informations complémentaires » puis « Exécuter quand même ».
- **Linux** : sous Wayland, la saisie directe demande une règle udev que l'application propose d'installer.
- **macOS** (13.4 ou plus récent) : l'application est signée ad hoc mais pas notarisée (pas de compte développeur Apple), macOS la bloque donc au premier lancement. Autorisez-la dans Réglages Système > Confidentialité et sécurité (« Ouvrir quand même »), ou exécutez `xattr -dr com.apple.quarantine /Applications/Plumed.app`.

## Fonctionnalités

- **Dictée par raccourci clavier** dans n'importe quelle application, avec aperçu en cours de dictée.
- **Pipeline de correction en 8 étapes** Voir [MEDICAL.fr.md](MEDICAL.fr.md).
- **Textes types** : une phrase dite est remplacée par un texte plus long.
- **Contexte pour les dictées courtes** : une dictée très brève est mieux reconnue grâce à la fin de la précédente.
- **PhoneMic** (expérimental, désactivé par défaut) : utiliser son smartphone comme micro, via un relais HTTPS chiffré de bout en bout (pas de stockage sur le téléphone, le relai ne peut rien déchiffrer, seul l'ordinateur a les données).
- **Tutoriel** intégré dans la barre latérale.

## Captures d'écran

![Écran d'accueil de Plumed : statistiques, visualisation du micro et dernière dictée](docs/screenshots/accueil.png)

## Confidentialité

La reconnaissance se fait sur votre ordinateur : l'audio et les transcriptions ne sont jamais envoyés sur internet. Les seules connexions réseau possibles sont listées dans **Paramètres > En ligne** : la vérification des mises à jour de l'application, le téléchargement du modèle, les mises à jour des données médicales (sur demande), les statistiques d'utilisation anonymes (facultatives, des totaux agrégés, jamais de texte ni d'audio) et le relais de PhoneMic. Voir la [politique de confidentialité](PRIVACY_POLICY.md).

## Licence

Plumed est un logiciel libre sous licence [GNU AGPL v3](LICENSE), comme Murmure dont il est issu.

## Remerciements et licences

- **[Murmure](https://www.murmure.app/)** par [Al1x-ai](https://www.al1x-ai.com/) : l'application dont Plumed est un fork (AGPL v3, [code source](https://github.com/Kieirra/murmure)). Soutenez le projet original !
- **[ParakeetWeb](https://github.com/thiswillbeyourgithub/parakeet_web)** : le mode médical porté ici, et la liste d'amplification de phrases médicales (AGPL v3).
- **[UltiMed](https://github.com/UltiMed-Org)** : les modèles de reconnaissance vocale [UltiMed v1](https://huggingface.co/Olicorne/parakeet-tdt-0.6b-v3-ultra-UltiMed-v1-onnx) et [UltiMed v2](https://huggingface.co/Olicorne/parakeet-tdt-0.6b-v3-ultra-UltiMed-v2-onnx) (CC BY 4.0), et ses [règles de correction des médicaments et termes médicaux](https://github.com/UltiMed-Org/UltiMed-ASR-FR-v2-regex-fixes) (AGPL-3.0-or-later), construites à partir des données ouvertes françaises sur le médicament (Licence Ouverte / Etalab 2.0 : OPEN_MEDIC, RETROCEDAM, BDPM).
- **[UltiMed-ASR-FR-v2-regex-fixes](https://github.com/UltiMed-Org/UltiMed-ASR-FR-v2-regex-fixes)** (liste `dictation_rules.csv`) : règles de dictée médicale (unités, constantes, ponctuation dictée), proposées par défaut comme règles de formatage (AGPL-3.0-or-later).
- **[LPSN](https://lpsn.dsmz.de/)** (List of Prokaryotic names with Standing in Nomenclature) : noms de bactéries inclus dans la liste d'amplification de phrases, sous licence [CC BY-SA 4.0](https://creativecommons.org/licenses/by-sa/4.0/). Citation requise : Parte, A.C., Sardà Carbasse, J., Meier-Kolthoff, J.P., Reimer, L.C. & Göker, M. (2020). List of Prokaryotic names with Standing in Nomenclature (LPSN) moves to the DSMZ. _International Journal of Systematic and Evolutionary Microbiology_ 70, 5607-5612. https://doi.org/10.1099/ijsem.0.004332
- **[Grammalecte](https://grammalecte.net)** par Olivier R. : le correcteur grammatical de l'étape 7 des corrections, version 2.3.0 (GNU GPL v3 ou ultérieure), inclus sans modification (sa version JavaScript, celle de l'extension Firefox) avec sa licence. Sources : [le paquet inclus](https://addons.mozilla.org/firefox/downloads/file/4643560/grammalecte_fr-2.3.0.xpi) et [l'archive des sources de la même version](https://grammalecte.net/zip/Grammalecte-fr-v2.3.0.zip). Détails dans [src-tauri/grammar-data/SOURCE.md](src-tauri/grammar-data/SOURCE.md).
- **[NVIDIA Parakeet TDT 0.6B v3](https://huggingface.co/nvidia/parakeet-tdt-0.6b-v3)** : le modèle de reconnaissance vocale.

Le détail des règles médicales incluses (provenance, mise à jour) est dans [src-tauri/medical-data/SOURCE.md](src-tauri/medical-data/SOURCE.md).
