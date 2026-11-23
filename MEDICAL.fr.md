# Dictée médicale (Plumed)

Fonctionnalités propres au fork Plumed, absentes de Murmure.

**Paramètres > Corrections** regroupe tout ce qui modifie le texte transcrit. Le texte passe par huit étapes, toujours dans cet ordre, une par onglet (le numéro de l'étape est affiché dans une pastille devant le nom de l'onglet) :

- **Explication** : une carte par étape, reliées par des flèches. Chaque carte montre un exemple avant et après, si l'étape est activée, et un bouton vers sa liste ou son option.
- **1 Vocabulaire médical** : les quelque 77 000 expressions vers lesquelles le modèle est orienté pendant la reconnaissance (« microangiopathie » rend « thrombotique » plus probable).
- **2 Mots qui se ressemblent** : les groupes de mots que Plumed départage en réécoutant le passage (« ituximab » devient « rituximab » après « instaure », « cetuximab » après « on passe »).
- **3 Dictionnaire personnel** : vos mots, favorisés pendant la reconnaissance puis écrits avec votre orthographe.
- **4 Nettoyage et uniformisation** : les options préinstallées (hésitations, mots répétés, correction de texte court, espace avant ? et !, nombres en chiffres, espace finale). La correction de texte court s'applique en fait après l'étape 5 et la mise en forme préinstallée après l'étape 6, ce qui est indiqué à côté d'elles.
- **5 Erreurs résiduelles** : les règles qui corrigent les erreurs fréquentes du modèle (« veine lafaxine » devient « venlafaxine »).
- **6 Formatage** : vos propres règles de mise en forme (au départ celles d'InterHop).
- **7 Grammaire** : le correcteur grammatical Grammalecte, qui n'applique que les corrections sûres des règles activées.
- **8 Textes types** : une phrase dite est remplacée par un texte plus long, éventuellement sur plusieurs lignes (« Insère l'examen clinique normal. » donne un examen clinique normal complet). Appliqué en dernier, après la grammaire, pour que le texte soit inséré tel quel, avec la même tolérance que les autres commandes vocales. Les textes fournis peuvent être lus et modifiés (la version modifiée devient la vôtre et l'originale est désactivée). L'exemple fourni est désactivé tant que vous ne l'activez pas ; vous pouvez ajouter, modifier et supprimer vos propres textes. Un onglet de plus, ajouté avec Claude Code.

Dans l'historique de la page d'accueil, la flèche d'une transcription montre ce que chaque étape a changé ; la correction de texte court et la mise en forme préinstallée y apparaissent sous l'étape 4.

Chaque liste se recherche (sans tenir compte des accents ni des majuscules), et chaque entrée préinstallée peut être désactivée ou complétée par les vôtres. Chaque onglet a aussi **Importer**, **Exporter** (dans le format de la liste : une expression par ligne pour le vocabulaire, JSONL comme les fichiers d'UltiMed pour les mots qui se ressemblent et les erreurs résiduelles, fichier `.murmure` pour les règles de mise en forme) et **Revenir à la liste d'origine**, qui supprime vos modifications après confirmation. Un import invalide (expression régulière incorrecte, groupe de moins de 2 candidats) est refusé en entier, avec la liste des erreurs.

Vos modifications des listes médicales sont enregistrées à part, une liste par fichier (`medical_rules.json`, `medical_phrases.json`, `medical_confusion_groups.json`), et conservées lors d'une mise à jour des règles médicales. L'export des paramètres (**Paramètres > Importer / Exporter**, fichier `.murmure`) les contient, une catégorie par onglet (Vocabulaire médical, Mots qui se ressemblent, Erreurs résiduelles) : seules vos modifications sont exportées, jamais les listes fournies. À l'import, chaque liste est vérifiée comme dans son onglet, et une liste invalide n'enregistre rien. Les fichiers `.murmure` plus anciens, sans ces catégories, s'importent comme avant.

## Ce qui passe par Internet

**Paramètres > En ligne** liste tout ce qui passe par le réseau : la vérification des mises à jour de l'application (à chaque démarrage, impossible à désactiver, rien n'est installé sans votre clic), le téléchargement du modèle de reconnaissance vocale (une seule fois, quand vous le choisissez, depuis Hugging Face ou à défaut depuis le relais Plumed `models.plumed.fr` ; il est conservé lors des mises à jour et se gère dans **Paramètres > Modèle**), la vérification des mises à jour des règles médicales et les statistiques d'utilisation anonymes (désactivées sauf si vous avez accepté, réglables dans cet onglet), et PhoneMic (seulement si vous l'activez). Ces réglages ne font jamais partie de l'export des paramètres.

## Correction des noms de médicaments et des termes médicaux

Activée par défaut : **Paramètres > Corrections > Erreurs résiduelles**, interrupteur en haut de l'onglet.

Environ 25 000 règles préinstallées corrigent les noms de médicaments et les termes médicaux que la reconnaissance vocale entend mal, par exemple « tacadol » devient « Takadol », « polllachiurie » devient « pollakiurie » et « soucrural » devient « sous-crural ». Ce sont les mêmes règles que le mode médical de [ParakeetWeb](http://parakeetweb.olicorne.org/?mode=med), appliquées en local, avant toutes les autres règles de formatage. Provenance et licence (AGPL-3.0-or-later) : [`src-tauri/medical-data/SOURCE.md`](src-tauri/medical-data/SOURCE.md).

Dans l'onglet **Erreurs résiduelles**, la section **Règles de correction** permet de :

- **Essayer les règles** : tapez une phrase pour voir ce que les règles en font.
- **Trouver une règle préinstallée** : cherchez par le mot correct (« Takadol ») ou par ce que la transcription a écrit (« tacadol »), puis modifiez le texte reconnu ou le remplacement, ou désactivez la règle.
- **Ajouter une règle** : un texte mal reconnu (expression régulière, sans distinction de casse, limites de mot ajoutées automatiquement) et son remplacement. Les règles ajoutées passent avant les règles préinstallées.
- **Vos modifications** montre tout ce que vous avez changé. Chaque règle peut être rétablie, et **Revenir à la liste d'origine** revient exactement aux règles préinstallées.

Ces modifications sont enregistrées à part (`medical_rules.json`), séparément de vos règles de mise en forme : importer ou réinitialiser l'un ne touche pas l'autre (dans l'export des paramètres, ce sont deux catégories distinctes). Pour corriger autre chose qu'un médicament (un nom propre, par exemple), utilisez plutôt les règles personnalisées.

## Contexte pour les dictées courtes

Activé par défaut : **Paramètres > Avancé > Contexte pour les dictées courtes**.

Une dictée très courte, par exemple un seul mot que l'on redicte pour le corriger, se reconnaît mal toute seule : le modèle n'a ni le timbre de votre voix ni la phrase autour. Quand une dictée dure moins de 2 secondes ou donne 3 mots au plus, Plumed la retranscrit derrière les 10 dernières secondes de la dictée précédente, séparées par 2 secondes de silence, puis ne garde que les mots prononcés après ce silence (repérés par leur horodatage, pas en comparant les textes).

Cet audio reste uniquement en mémoire vive, n'est jamais écrit sur le disque, et est oublié 5 minutes après la dernière dictée, à la fermeture de Plumed, ou dès que l'option est désactivée.

## PhoneMic (dicter depuis son téléphone, alpha)

> **Alpha :** cette fonctionnalité est une préversion, attendez-vous à des bugs.

Désactivé par défaut : **PhoneMic** dans la barre latérale.

Avec PhoneMic, le téléphone sert de micro, même s'il n'est pas sur le même réseau que l'ordinateur (4G, Wi-Fi invité d'un établissement) :

1. Cliquer sur **Connecter un téléphone** : un QR code s'affiche.
2. Le scanner avec l'appareil photo du téléphone. La page PhoneMic de Plumed s'ouvre (en français, ou en anglais), sans rien installer.
3. Les deux écrans affichent un code (par exemple `524C-F8E8-D5A4-2D82`). S'il est identique, confirmer sur l'ordinateur.
4. Dicter sur le téléphone : le texte transcrit est collé dans l'application active de l'ordinateur, comme pour une dictée normale. En mode mains libres, le téléphone écoute en continu et n'envoie que la parole, jamais les silences.

L'audio passe par un serveur relais (`https://turn.plumed.fr` par défaut, modifiable), mais il est chiffré de bout en bout entre le téléphone et Plumed : le relais ne peut pas l'écouter. La vérification du code empêche un relais malveillant de s'intercaler. Cette garantie suppose que le relais sert une page téléphone non modifiée (la page et son code de chiffrement viennent du relais) : n'utilisez qu'un relais de confiance, ou hébergez le vôtre. Le relais voit en revanche les adresses IP et la taille des paquets chiffrés. Le téléphone se reconnecte tout seul après une coupure réseau, mais recharger la page demande de scanner à nouveau le QR code. Une fois connecté, le bouton « Appairer un autre téléphone » réaffiche le QR code pour changer de téléphone.

Seul le relais HTTPS est utilisé (pas de WebRTC, souvent bloqué sur les réseaux d'entreprise ou d'établissement), et le proxy système est respecté.

## Règles de dictée InterHop (installées par défaut)

L'association [InterHop](https://interhop.org/) maintient des règles de dictée médicale pour Murmure (unités, constantes, ponctuation dictée, modèles d'examen déclenchés par « ajoute la commande ... ») : [framagit.org/interhop/murmure-regex](https://framagit.org/interhop/murmure-regex). Par exemple « tension 120 80 » devient « TA : 120/80 » et « 3 fois par jour » devient « 3/j ».

Une nouvelle installation de Plumed les contient déjà dans **Paramètres > Corrections > Formatage**, comme règles personnalisées ordinaires : vous pouvez les modifier, les désactiver ou les supprimer, et ajouter les vôtres. Elles s'appliquent après la correction des médicaments. **Revenir à la liste d'origine** remet exactement ces règles.

Sur une installation existante (ou après les avoir supprimées), vous pouvez les importer :

1. Télécharger [`config.murmure`](https://framagit.org/interhop/murmure-regex/-/raw/main/config.murmure).
2. Dans **Paramètres > Importer / Exporter**, importer ce fichier avec la stratégie **Fusionner**, pour garder vos propres règles (**Remplacer** les supprime). Les règles InterHop déjà présentes sont mises à jour, pas dupliquées.

En ligne de commande : `plumed import config.murmure --strategy merge`. Sur un poste verrouillé, l'option **Paramètres > Avancé > Bloquer l'import en ligne de commande** refuse cette commande.

Attention : l'import, quelle que soit la stratégie, applique aussi les options préinstallées d'InterHop (espace avant ? et !, conversion des nombres en chiffres en français, etc.). La correction des médicaments reste activée.

Rédigé avec Claude Code.
