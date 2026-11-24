<p align="center"><img src="src-tauri/icons/128x128@2x.png" alt="Icône de Plumed" width="128"></p>

# Plumed

Plumed est une application de dictée vocale respectueuse de la confidentialité, réalisée par [Olivier Cornelis](https://olicorne.org/fr), interne en psychiatrie et développeur.

Plumed n'est pas un projet commercial et est donc **entièrement gratuite et sans pub**, ne nécessite **pas de compte**, ne piste pas ses utilisateurs, **tourne directement sur votre ordinateur**, **fonctionne hors ligne** une fois le modèle téléchargé et permet de dicter dans **n'importe quelle application** (logiciel métier, traitement de texte, messagerie, etc.), avec une reconnaissance adaptée au **vocabulaire médical français** (médicaments, termes médicaux, comptes rendus, ...).

Plumed n'est cependant pas un dispositif médical certifié : l'application est fournie telle quelle, sans garantie autre que sa [politique de confidentialité](PRIVACY_POLICY.md). De principe, il convient de ne pas utiliser d'information identifiante (noms de patients, ...) dans Plumed.

## Sommaire

- [Installation](#installation)
- [Fonctionnalités](#fonctionnalités)
- [Captures d'écran](#captures-décran)
- [Confidentialité](#confidentialité)
- [Remerciements et licences](#remerciements-et-licences)

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
- **Fonctionne hors ligne** : la reconnaissance et les corrections tournent sur l'ordinateur, internet ne sert qu'aux fonctions facultatives listées dans **Paramètres > En ligne**.
- **Corrections en 8 étapes** adaptées au vocabulaire médical, détaillées ci-dessous.
- **Textes types** : une phrase dite est remplacée par un texte plus long, utile pour les modèles répétitifs.
- **PhoneMic** (expérimental) : le téléphone sert de micro, l'audio étant chiffré de bout en bout jusqu'à l'ordinateur.
- **Tutoriel** intégré dans la barre latérale.

### Corrections

**Paramètres > Corrections** regroupe tout ce qui modifie le texte transcrit. Le texte passe par huit étapes, toujours dans cet ordre, une par onglet (le numéro de l'étape est affiché dans une pastille devant le nom de l'onglet) :

- **Explications** : une carte par étape, reliées par des flèches. Chaque carte montre un exemple avant et après, si l'étape est activée, et un bouton vers sa liste ou son option.
- **1 Vocabulaire médical** : les quelque 77 000 expressions vers lesquelles le modèle est orienté pendant la reconnaissance (« microangiopathie » rend « thrombotique » plus probable).
- **2 Mots qui se ressemblent** : les groupes de mots que Plumed départage en réécoutant le passage (« ituximab » devient « rituximab » après « instaure », « cetuximab » après « on passe »).
- **3 Dictionnaire personnel** : vos mots, favorisés pendant la reconnaissance puis écrits avec votre orthographe.
- **4 Nettoyage et uniformisation** : les options préinstallées (hésitations, mots répétés, correction de texte court, espace avant ? et !, nombres en chiffres, espace finale). La correction de texte court s'applique en fait après l'étape 5 et la mise en forme préinstallée après l'étape 6, ce qui est indiqué à côté d'elles.
- **5 Erreurs résiduelles** : les règles qui corrigent les erreurs fréquentes du modèle (« veine lafaxine » devient « venlafaxine »).
- **6 Formatage** : vos propres règles de mise en forme (au départ les règles de dictée d'UltiMed).
- **7 Grammaire** : le correcteur grammatical Grammalecte, qui n'applique que les corrections sûres des règles activées.
- **8 Textes types** : une phrase dite est remplacée par un texte plus long, éventuellement sur plusieurs lignes (« Insère l'examen clinique normal. » donne un examen clinique normal complet). Appliqué en dernier, après la grammaire, pour que le texte soit inséré tel quel, avec la même tolérance que les autres commandes vocales. Les textes fournis peuvent être lus et modifiés (la version modifiée devient la vôtre et l'originale est désactivée). L'exemple fourni est désactivé tant que vous ne l'activez pas ; vous pouvez ajouter, modifier et supprimer vos propres textes.

Dans l'historique de la page d'accueil, chaque transcription est abrégée (les retours à la ligne y sont affichés « ↵ ») ; sa flèche montre le texte complet et ce que chaque étape a changé ; la correction de texte court et la mise en forme préinstallée y apparaissent sous l'étape 4.

Chaque liste se recherche (sans tenir compte des accents ni des majuscules), et chaque entrée préinstallée peut être désactivée ou complétée par les vôtres. Chaque onglet a aussi **Importer**, **Exporter** (dans le format de la liste : une expression par ligne pour le vocabulaire, JSONL comme les fichiers d'UltiMed pour les mots qui se ressemblent et les erreurs résiduelles, fichier `.murmure` pour les règles de mise en forme) et **Revenir à la liste d'origine**, qui supprime vos modifications après confirmation. Un import invalide (expression régulière incorrecte, groupe de moins de 2 candidats) est refusé en entier, avec la liste des erreurs.

Vos modifications des listes médicales sont enregistrées à part, une liste par fichier (`medical_rules.json`, `medical_phrases.json`, `medical_confusion_groups.json`), et conservées lors d'une mise à jour des règles médicales. L'export des paramètres (**Paramètres > Importer / Exporter**, fichier `.murmure`) les contient, une catégorie par onglet (Vocabulaire médical, Mots qui se ressemblent, Erreurs résiduelles, Grammaire, Textes types) : seules vos modifications sont exportées, jamais les listes fournies. À l'import, chaque liste est vérifiée comme dans son onglet, et une liste invalide n'enregistre rien. Les fichiers `.murmure` plus anciens, sans ces catégories, s'importent comme avant.

### Correction des noms de médicaments et des termes médicaux

Activée par défaut : **Paramètres > Corrections > Erreurs résiduelles**, interrupteur en haut de l'onglet.

Environ 45 000 règles préinstallées corrigent les noms de médicaments et les termes médicaux que la reconnaissance vocale entend mal, par exemple « tacadol » devient « Takadol », « polllachiurie » devient « pollakiurie » et « soucrural » devient « sous-crural ». Ce sont les mêmes règles que le mode médical de [ParakeetWeb](http://parakeetweb.olicorne.org/?mode=med), appliquées en local, avant toutes les autres règles de formatage. Provenance et licence (AGPL-3.0-or-later) : [`src-tauri/medical-data/SOURCE.md`](src-tauri/medical-data/SOURCE.md).

Dans l'onglet **Erreurs résiduelles**, la section **Règles de correction** permet de :

- **Essayer les règles** : tapez une phrase pour voir ce que les règles en font.
- **Trouver une règle préinstallée** : cherchez par le mot correct (« Takadol ») ou par ce que la transcription a écrit (« tacadol »), puis modifiez le texte reconnu ou le remplacement, ou désactivez la règle.
- **Ajouter une règle** : un texte mal reconnu (expression régulière, sans distinction de casse, limites de mot ajoutées automatiquement) et son remplacement. Les règles ajoutées passent avant les règles préinstallées.
- **Vos modifications** montre tout ce que vous avez changé. Chaque règle peut être rétablie, et **Revenir à la liste d'origine** revient exactement aux règles préinstallées.

Ces modifications sont enregistrées à part (`medical_rules.json`), séparément de vos règles de mise en forme : importer ou réinitialiser l'un ne touche pas l'autre (dans l'export des paramètres, ce sont deux catégories distinctes). Pour corriger autre chose qu'un médicament (un nom propre, par exemple), utilisez plutôt les règles personnalisées.

### Contexte pour les dictées courtes

Activé par défaut : **Paramètres > Avancé > Contexte pour les dictées courtes**.

Une dictée très courte, par exemple un seul mot que l'on redicte pour le corriger, se reconnaît mal toute seule : le modèle n'a ni le timbre de votre voix ni la phrase autour. Quand une dictée dure moins de 2 secondes ou donne 3 mots au plus, Plumed la retranscrit derrière les 10 dernières secondes de la dictée précédente, séparées par 2 secondes de silence, puis ne garde que les mots prononcés après ce silence (repérés par leur horodatage, pas en comparant les textes).

Cet audio reste uniquement en mémoire vive, n'est jamais écrit sur le disque, et est oublié 5 minutes après la dernière dictée, à la fermeture de Plumed, ou dès que l'option est désactivée.

### PhoneMic (dicter depuis son téléphone, expérimental)

> **Expérimental :** cette fonctionnalité est une préversion, attendez-vous à des bugs, et évitez d'y faire passer des contenus sensibles.

Désactivé par défaut : **PhoneMic** dans la barre latérale.

Avec PhoneMic, le téléphone sert de micro, même s'il n'est pas sur le même réseau que l'ordinateur (4G, Wi-Fi invité d'un établissement) :

1. Cliquer sur **Connecter un téléphone** : un QR code s'affiche.
2. Le scanner avec l'appareil photo du téléphone. La page PhoneMic de Plumed s'ouvre (en français, ou en anglais), sans rien installer.
3. Les deux écrans affichent un code (par exemple `524C-F8E8-D5A4-2D82`). S'il est identique, confirmer sur l'ordinateur.
4. Dicter sur le téléphone : le texte transcrit est collé dans l'application active de l'ordinateur, comme pour une dictée normale. En mode mains libres, le téléphone écoute en continu et n'envoie que la parole, jamais les silences.

L'audio passe par un serveur relais (`https://turn.plumed.fr` par défaut, modifiable), mais il est chiffré de bout en bout entre le téléphone et Plumed : le relais ne peut pas l'écouter. La vérification du code empêche un relais malveillant de s'intercaler. Cette garantie suppose que le relais sert une page téléphone non modifiée (la page et son code de chiffrement viennent du relais) : n'utilisez qu'un relais de confiance, ou hébergez le vôtre. Le relais voit en revanche les adresses IP et la taille des paquets chiffrés. Le téléphone se reconnecte tout seul après une coupure réseau, mais recharger la page demande de scanner à nouveau le QR code. Une fois connecté, le bouton « Appairer un autre téléphone » réaffiche le QR code pour changer de téléphone.

Seul le relais HTTPS est utilisé (pas de WebRTC, souvent bloqué sur les réseaux d'établissement), et le proxy système est respecté.

### Règles de dictée (installées par défaut)

Une nouvelle installation de Plumed contient 45 règles de dictée médicale (unités, constantes, ponctuation et mise en page dictées) : la liste `dictation_rules.csv` d'[UltiMed-ASR-FR-v2-regex-fixes](https://github.com/UltiMed-Org/UltiMed-ASR-FR-v2-regex-fixes). Par exemple « tension 120 80 » devient « TA : 120/80 » et « 3 fois par jour » devient « 3/j ».

Elles se trouvent dans **Paramètres > Corrections > Formatage**, comme règles personnalisées ordinaires : vous pouvez les modifier, les désactiver ou les supprimer, et ajouter les vôtres. Elles s'appliquent après la correction des médicaments. **Revenir à la liste d'origine** remet exactement ces règles.

## Captures d'écran

![Écran d'accueil de Plumed : statistiques, visualisation du micro et dernière dictée](docs/screenshots/accueil.png)

## Confidentialité

La reconnaissance se fait sur votre ordinateur : l'audio et les transcriptions ne sont jamais envoyés sur internet. Une fois le modèle de reconnaissance vocale téléchargé, Plumed fonctionne entièrement hors ligne : aucune des connexions ci-dessous n'est nécessaire pour dicter.

**Paramètres > En ligne** liste tout ce qui passe par le réseau : la vérification des mises à jour de l'application (à chaque démarrage, impossible à désactiver, rien n'est installé sans votre clic), le téléchargement du modèle de reconnaissance vocale (une seule fois, quand vous le choisissez, depuis Hugging Face ou à défaut depuis le relais Plumed `models.plumed.fr` ; il est conservé lors des mises à jour et se gère dans **Paramètres > Modèle**), la vérification des mises à jour des règles médicales (rien n'est téléchargé sans votre clic ; « Voir les changements » dans la fenêtre de mise à jour montre les lignes ajoutées et retirées avant d'installer) et les statistiques d'utilisation anonymes (des totaux agrégés, jamais de texte ni d'audio ; désactivées sauf si vous avez accepté, réglables dans cet onglet), et PhoneMic (seulement si vous l'activez). Ces réglages ne font jamais partie de l'export des paramètres. Voir la [politique de confidentialité](PRIVACY_POLICY.md).

## Remerciements et licences

Plumed est initialement un fork de [Murmure](https://github.com/Kieirra/murmure) qui intègre les fonctionnalités du mode médical de [ParakeetWeb](http://parakeetweb.olicorne.org/?mode=med) et les modèles [UltiMed](https://github.com/UltiMed-Org). UltiMed et ParakeetWeb sont du même auteur que Plumed.

Plumed est un logiciel libre sous licence [GNU AGPL v3](LICENSE), comme Murmure.

- **[Murmure](https://www.murmure.app/)** par [Al1x-ai](https://www.al1x-ai.com/) : l'application dont Plumed est un fork (AGPL v3, [code source](https://github.com/Kieirra/murmure)). Soutenez le projet original !
- **[ParakeetWeb](https://github.com/thiswillbeyourgithub/parakeet_web)** : dont le mode médical était un précurseur de Plumed (AGPL v3).
- **[UltiMed](https://github.com/UltiMed-Org)** : les modèles de reconnaissance vocale [UltiMed v1](https://huggingface.co/Olicorne/parakeet-tdt-0.6b-v3-ultra-UltiMed-v1-onnx) et [UltiMed v2](https://huggingface.co/Olicorne/parakeet-tdt-0.6b-v3-ultra-UltiMed-v2-onnx) (CC BY 4.0), et ses [règles de correction des médicaments et termes médicaux](https://github.com/UltiMed-Org/UltiMed-ASR-FR-v2-regex-fixes) (AGPL-3.0-or-later), construites à partir des données ouvertes françaises sur le médicament.
- **[UltiMed-ASR-FR-v2-regex-fixes](https://github.com/UltiMed-Org/UltiMed-ASR-FR-v2-regex-fixes)** (liste `dictation_rules.csv`) : règles de dictée médicale (unités, constantes, ponctuation dictée), proposées par défaut comme règles de formatage (AGPL-3.0-or-later).
- **[LPSN](https://lpsn.dsmz.de/)** (List of Prokaryotic names with Standing in Nomenclature) : noms de bactéries inclus dans la liste d'amplification de phrases, sous licence [CC BY-SA 4.0](https://creativecommons.org/licenses/by-sa/4.0/). Citation requise : Parte, A.C., Sardà Carbasse, J., Meier-Kolthoff, J.P., Reimer, L.C. & Göker, M. (2020). List of Prokaryotic names with Standing in Nomenclature (LPSN) moves to the DSMZ. _International Journal of Systematic and Evolutionary Microbiology_ 70, 5607-5612. https://doi.org/10.1099/ijsem.0.004332
- **[Grammalecte](https://grammalecte.net)** par Olivier R. : le correcteur grammatical de l'étape 7 des corrections, version 2.3.0 (GNU GPL v3 ou ultérieure), inclus sans modification (sa version JavaScript, celle de l'extension Firefox) avec sa licence. Sources : [le paquet inclus](https://addons.mozilla.org/firefox/downloads/file/4643560/grammalecte_fr-2.3.0.xpi) et [l'archive des sources de la même version](https://grammalecte.net/zip/Grammalecte-fr-v2.3.0.zip). Détails dans [src-tauri/grammar-data/SOURCE.md](src-tauri/grammar-data/SOURCE.md).

Le détail des règles médicales incluses (provenance, mise à jour) est dans [src-tauri/medical-data/SOURCE.md](src-tauri/medical-data/SOURCE.md).
