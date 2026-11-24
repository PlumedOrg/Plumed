---
title: Corrections
description: Les huit étapes qui corrigent et mettent en forme le texte transcrit.
sidebar:
  order: 2
---

**Paramètres > Corrections** regroupe tout ce qui modifie le texte transcrit. Le texte passe par huit étapes, toujours dans cet ordre, une par onglet (le numéro de l'étape est affiché dans une pastille devant le nom de l'onglet). L'onglet **Explications** montre une carte par étape, avec un exemple avant et après.

1. **Vocabulaire médical** : les quelque 77 000 expressions vers lesquelles le modèle est orienté pendant la reconnaissance (« microangiopathie » rend « thrombotique » plus probable). En bas de l'onglet, le **poids** et la **marge (min-p)** du renforcement sont réglables (vides, ils gardent les valeurs d'origine de la liste) et un bouton rétablit ces valeurs ; ils font partie de l'export des paramètres.
2. **Mots qui se ressemblent** : les groupes de mots que Plumed départage en réécoutant le passage (« ituximab » devient « rituximab » après « instaure », « cetuximab » après « on passe »).
3. **Dictionnaire personnel** : vos mots, favorisés pendant la reconnaissance puis écrits avec votre orthographe.
4. **Nettoyage et uniformisation** : les options préinstallées (hésitations, mots répétés, correction de texte court, espace avant ? et !, nombres en chiffres, espace finale). La correction de texte court s'applique en fait après l'étape 5 et la mise en forme préinstallée après l'étape 6, ce qui est indiqué à côté d'elles.
5. **Erreurs résiduelles** : les règles qui corrigent les erreurs fréquentes du modèle (« veine lafaxine » devient « venlafaxine »). Voir [Médicaments et termes médicaux](/docs/fonctionnalites/medicaments/).
6. **Formatage** : vos propres règles de mise en forme, au départ les [règles de dictée](#règles-de-dictée) d'UltiMed.
7. **Grammaire** : le correcteur grammatical Grammalecte, qui n'applique que les corrections sûres des règles activées.
8. **Textes types** : voir [Textes types](/docs/fonctionnalites/textes-types/).

## Modifier les listes

Chaque liste se recherche (sans tenir compte des accents ni des majuscules), et chaque entrée préinstallée peut être désactivée ou complétée par les vôtres. Chaque onglet a aussi :

- **Importer** et **Exporter**, dans le format de la liste : une expression par ligne pour le vocabulaire, JSONL comme les fichiers d'UltiMed pour les mots qui se ressemblent et les erreurs résiduelles, fichier `.murmure` pour les règles de mise en forme. Un import invalide (expression régulière incorrecte, groupe de moins de 2 candidats) est refusé en entier, avec la liste des erreurs.
- **Revenir à la liste d'origine**, qui supprime vos modifications après confirmation.

Vos modifications des listes médicales sont enregistrées à part et conservées lors d'une mise à jour des règles médicales.

## Règles de dictée

Une nouvelle installation contient 45 règles de dictée médicale (unités, constantes, ponctuation et mise en page dictées), la liste `dictation_rules.csv` d'[UltiMed-ASR-FR-v2-regex-fixes](https://github.com/UltiMed-Org/UltiMed-ASR-FR-v2-regex-fixes). Par exemple :

| Vous dites | Plumed écrit |
| --- | --- |
| 15 mg par kilo par jour | 15 mg/kg/j |
| 3 fois par jour | 3/j |

Elles se trouvent dans **Paramètres > Corrections > Formatage**, comme règles personnalisées ordinaires : vous pouvez les modifier, les désactiver ou les supprimer, et ajouter les vôtres. **Revenir à la liste d'origine** remet exactement ces règles.

## Exporter et importer ses réglages

**Paramètres > Importer / Exporter** enregistre vos réglages dans un fichier `.murmure`, avec une catégorie par onglet des corrections. Seules vos modifications sont exportées, jamais les listes fournies, et jamais les réglages réseau. À l'import, chaque liste est vérifiée comme dans son onglet, et l'aperçu détaille les entrées ajoutées, modifiées et désactivées avant d'appliquer.
