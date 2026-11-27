---
title: Corrections
description: Comment Plumed passe de ce que vous dites au texte final, étape par étape.
sidebar:
  order: 2
---

Plumed utilise les modèles de reconnaissance vocale [UltiMed](https://github.com/UltiMed-Org), entraînés sur du vocabulaire médical, mais aucun modèle ne connaît parfaitement notre vocabulaire. Dictez « venlafaxine » et il peut écrire « veine la faxine » ; dictez « 15 mg par kilo par jour » et il l'écrit en toutes lettres. Ces erreurs reviennent toujours de la même façon, donc on peut les prévoir.

Plumed intervient à deux moments :

- **pendant la reconnaissance**, il aide le modèle à choisir le bon mot, en lui donnant une liste de termes médicaux à privilégier ;
- **après la reconnaissance**, il corrige et met en forme le texte écrit, avec des listes de corrections connues.

Tout se règle dans **Paramètres > Corrections**, avec un onglet par étape. Chaque liste fournie est un simple fichier texte, que vous pouvez lire sur GitHub (liens ci-dessous) ; dans l'application, vous pouvez désactiver une entrée ou ajouter les vôtres.

## Les étapes en bref

Le texte passe par ces étapes, dans cet ordre :

| Étape | Ce qu'elle fait | Exemple |
| --- | --- | --- |
| 1. Vocabulaire médical | oriente le modèle vers 77 000 termes médicaux | « thrombotique » plus probable après « microangiopathie » |
| 2. Mots qui se ressemblent | départage des mots proches selon le contexte | « étuximab » devient « rituximab » ou « cetuximab » |
| 3. Dictionnaire personnel | ajoute vos propres mots | le nom de votre service |
| 4. Nettoyage | retire les hésitations, écrit les nombres en chiffres | « euh trois comprimés » devient « 3 comprimés » |
| 5. Erreurs résiduelles | corrige les erreurs fréquentes du modèle | « veine la faxine » devient « venlafaxine » |
| 6. Formatage | applique les règles de dictée | « 2 comprimés » devient « 2 cp » |
| 7. Grammaire | corrige les fautes sûres | accords, espaces |
| 8. Textes types | insère un texte préparé | « Insère l'examen clinique normal. » |

Le détail de chaque étape suit.

## Vocabulaire médical

Avant même d'écrire, le modèle hésite souvent entre plusieurs mots qui sonnent pareil. Plumed lui donne une liste d'environ 77 000 termes et expressions médicales (maladies, médicaments, anatomie, bactéries...) qu'il doit privilégier. Après « microangiopathie », par exemple, « thrombotique » devient plus probable.

- La liste : [french_medical.txt](https://github.com/PlumedOrg/Plumed/blob/medmain/src-tauri/medical-data/phrase-boosting/french_medical.txt), une expression par ligne (exemples : « Entamoeba hartmanni », « crête uréthrale de la femme »).
- Réglages : en bas de l'onglet, le **poids** (la force du coup de pouce) et la **marge (min-p)** (jusqu'où un terme peu probable peut être repêché). Laissés vides, ils gardent les valeurs prévues pour la liste, et un bouton les rétablit.

## Mots qui se ressemblent

Certains mots sont si proches que le modèle ne peut pas trancher seul : il entend « étuximab », qui peut être « cetuximab » ou « rituximab ». Pour ces cas, Plumed réécoute le passage avec chaque candidat et garde celui qui colle le mieux au contexte : « rituximab » après « on instaure », « cetuximab » après « on passe au ».

- La liste : [confusion_groups.jsonl](https://github.com/PlumedOrg/Plumed/blob/medmain/src-tauri/medical-data/regex-fixes/confusion_groups.jsonl), environ 400 groupes. Chaque ligne donne le mot entendu et ses candidats, par exemple `{"heard": "torvastatine", "candidates": [{"text": "atorvastatine"}, ...]}`.

## Dictionnaire personnel

Vos mots à vous : noms de collègues, de services, de médicaments rares. Ils sont favorisés pendant la reconnaissance, puis écrits avec votre orthographe. Cette liste est vide au départ.

## Nettoyage et uniformisation

Des options simples, à cocher : retirer les hésitations (« euh ») et les mots répétés, écrire les nombres en chiffres, mettre une espace avant ? et !, ajouter une espace finale, corriger les textes très courts.

Deux de ces options s'appliquent en fait plus loin, ce qui est indiqué à côté d'elles : la correction des textes courts après l'étape 5, et la mise en forme après l'étape 6.

## Erreurs résiduelles

Même aidé, le modèle se trompe encore, et souvent de la même façon. Ces erreurs ont été relevées sur des milliers de termes médicaux passés au modèle, puis transformées en règles de correction (environ 45 000) : « veine la faxine », « veine lofaxine » ou « venlofaxine » deviennent « venlafaxine », « par acétamol » devient « paracétamol ».

Les règles viennent du projet [UltiMed-ASR-FR-v2-regex-fixes](https://github.com/UltiMed-Org/UltiMed-ASR-FR-v2-regex-fixes), en plusieurs fichiers selon leur origine :

| Fichier | Contenu |
| --- | --- |
| [drug_fix_rules.jsonl](https://github.com/PlumedOrg/Plumed/blob/medmain/src-tauri/medical-data/regex-fixes/drug_fix_rules.jsonl) | noms de médicaments |
| [dictionary_fix_rules.jsonl](https://github.com/PlumedOrg/Plumed/blob/medmain/src-tauri/medical-data/regex-fixes/dictionary_fix_rules.jsonl) | termes du dictionnaire médical (le plus gros fichier) |
| [parhaf_fix_rules.jsonl](https://github.com/PlumedOrg/Plumed/blob/medmain/src-tauri/medical-data/regex-fixes/parhaf_fix_rules.jsonl) | termes du corpus médical PARHAF |
| [acronyms_fix_rules.jsonl](https://github.com/PlumedOrg/Plumed/blob/medmain/src-tauri/medical-data/regex-fixes/acronyms_fix_rules.jsonl) | sigles |
| [parrot_fix_rules.jsonl](https://github.com/PlumedOrg/Plumed/blob/medmain/src-tauri/medical-data/regex-fixes/parrot_fix_rules.jsonl) | termes du corpus PARROT |
| [manual_fix_rules.jsonl](https://github.com/PlumedOrg/Plumed/blob/medmain/src-tauri/medical-data/regex-fixes/manual_fix_rules.jsonl) | règles écrites à la main |
| [plumed_fix_rules.jsonl](https://github.com/PlumedOrg/Plumed/blob/medmain/src-tauri/medical-data/plumed-rules/plumed_fix_rules.jsonl) | règles propres à Plumed, hors UltiMed (par exemple « plume med » devient « Plumed »), appliquées en dernier |

Chaque ligne contient le mot correct (`replacement`), les formes erronées entendues (`variants`) et une expression régulière (`pattern`) qui les reconnaît, avec ou sans accents. Ces règles sont mises à jour sans attendre une nouvelle version de Plumed : voir [Mises à jour des règles](#mises-à-jour-des-règles).

Activée par défaut : **Paramètres > Corrections > Erreurs résiduelles**, interrupteur en haut de l'onglet.

Quelques exemples :

| Transcription brute | Après correction |
| --- | --- |
| tacadol | Takadol |
| polllachiurie | pollakiurie |
| soucrural | sous-crural |

Ce sont les mêmes règles que le mode médical de [ParakeetWeb](http://parakeetweb.olicorne.org/?mode=med), appliquées en local. Provenance et licence (AGPL-3.0-or-later) : [`SOURCE.md`](https://github.com/PlumedOrg/Plumed/blob/medmain/src-tauri/medical-data/SOURCE.md).

### Règles de correction

Dans l'onglet **Erreurs résiduelles**, la section **Règles de correction** permet de :

- **Essayer les règles** : tapez une phrase pour voir ce que les règles en font.
- **Trouver une règle préinstallée** : cherchez par le mot correct (« Takadol ») ou par ce que la transcription a écrit (« tacadol »), puis modifiez le texte reconnu ou le remplacement, ou désactivez la règle.
- **Ajouter une règle** : un texte mal reconnu (expression régulière, sans distinction de casse, limites de mot ajoutées automatiquement) et son remplacement. Les règles ajoutées passent avant les règles préinstallées.
- **Vos modifications** : tout ce que vous avez changé, chaque règle pouvant être rétablie.

Ces modifications sont enregistrées séparément de vos règles de mise en forme : importer ou réinitialiser l'un ne touche pas l'autre. Pour corriger autre chose qu'un médicament (un nom propre, par exemple), utilisez plutôt les règles de [Formatage](#formatage).

### Mises à jour des règles

Si vous l'avez accepté au premier lancement (modifiable dans **Paramètres > En ligne**), Plumed vérifie au démarrage, puis une fois par jour s'il reste ouvert, s'il existe de nouvelles règles médicales. Rien n'est téléchargé sans votre clic, et « Voir les changements » montre les lignes ajoutées et retirées avant d'installer.

## Formatage

Vos règles de mise en forme : « dire ceci » donne « écrire cela ». Au départ, l'onglet contient les règles de dictée ci-dessous, que vous pouvez modifier, désactiver ou supprimer, et compléter par les vôtres.

### Règles de dictée

Une nouvelle installation contient 47 règles de dictée médicale : unités, constantes, ponctuation et mise en page dictées. Quelques exemples :

| Vous dites | Plumed écrit |
| --- | --- |
| 15 mg par kilo par jour | 15 mg/kg/j |
| 3 fois par jour | 3/j |
| 2 comprimés | 2 cp |
| température 38 2 | T° 38,2 |
| mètre carré | m2 |
| ouvrez la parenthèse | ( |
| point d'interrogation | ? |
| saute une ligne | (retour à la ligne) |

- La liste : [dictation_rules.csv](https://github.com/PlumedOrg/Plumed/blob/medmain/src-tauri/medical-data/regex-fixes/dictation_rules.csv), d'UltiMed, avec pour chaque règle un exemple avant et après.
- **Revenir à la liste d'origine** remet exactement ces 47 règles.

## Grammaire

Le correcteur [Grammalecte](https://grammalecte.net) relit le texte. Plumed n'applique que ses corrections sûres (celles qui n'ont qu'une seule proposition), et seulement pour les règles cochées dans l'onglet.

## Textes types

Une phrase déclencheuse insère un texte préparé. Dictez « Insère l'examen clinique normal. » et Plumed écrit un examen clinique normal complet, à compléter.

- Appliqués en dernier, après la grammaire, pour que le texte soit inséré tel quel, avec la même tolérance que les autres commandes vocales.
- Les textes fournis peuvent être lus et modifiés : la version modifiée devient la vôtre et l'originale est désactivée.
- L'exemple fourni est désactivé tant que vous ne l'activez pas (le [Tutoriel](/docs/fonctionnalites/dictee/) propose de l'essayer).
- Vous pouvez ajouter, modifier et supprimer vos propres textes.

La liste fournie est [templates.jsonl](https://github.com/PlumedOrg/Plumed/blob/medmain/src-tauri/medical-data/text-templates/templates.jsonl).

## Modifier les listes

Dans chaque onglet :

- la **recherche** ignore les accents et les majuscules ;
- chaque entrée fournie peut être **désactivée**, et vous pouvez **ajouter** les vôtres ;
- **Importer** et **Exporter** utilisent le format des fichiers ci-dessus (une expression par ligne pour le vocabulaire, JSONL pour les mots qui se ressemblent et les erreurs résiduelles, fichier `.murmure` pour le formatage). Un import invalide, par exemple une expression régulière incorrecte ou un groupe de moins de 2 candidats, est refusé en entier, avec la liste des erreurs ;
- **Revenir à la liste d'origine** efface vos modifications, après confirmation.

Vos modifications sont enregistrées à part : une mise à jour des règles médicales ne les écrase pas.

## Exporter et importer ses réglages

Pour retrouver vos réglages sur un autre ordinateur ou les partager avec un collègue : **Paramètres > Importer / Exporter** crée un fichier `.murmure`, avec une catégorie par onglet des corrections. Seules vos modifications y sont, jamais les listes fournies, et jamais les réglages réseau. À l'import, chaque liste est vérifiée comme dans son onglet, et un aperçu montre les entrées ajoutées, modifiées et désactivées avant d'appliquer.
