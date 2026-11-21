# Dictée médicale (Plumed)

Fonctionnalités propres au fork Plumed, absentes de Murmure.

**Paramètres > Corrections** regroupe tout ce qui modifie le texte transcrit, en trois onglets : **Explication** (chaque étape de correction dans l'ordre où elle s'applique, si elle est activée, et un bouton vers son réglage), **Dictionnaire personnel** et **Règles**. L'onglet Règles n'affiche par défaut que vos propres règles ; l'interrupteur **Afficher les règles intégrées** (mémorisé) ajoute les options intégrées, les mises à jour des données médicales et la recherche dans les règles intégrées.

## Correction des noms de médicaments et des termes médicaux

Activée par défaut : **Paramètres > Corrections > Règles**, avec **Afficher les règles intégrées**, option **Corriger les noms de médicaments et les termes médicaux**.

Environ 25 000 règles intégrées corrigent les noms de médicaments et les termes médicaux que la reconnaissance vocale entend mal, par exemple « tacadol » devient « Takadol », « polllachiurie » devient « pollakiurie » et « soucrural » devient « sous-crural ». Ce sont les mêmes règles que le mode médical de [ParakeetWeb](http://parakeetweb.olicorne.org/?mode=med), appliquées en local, avant toutes les autres règles de formatage. Provenance et licence (AGPL-3.0-or-later) : [`src-tauri/medical-data/SOURCE.md`](src-tauri/medical-data/SOURCE.md).

Dans l'onglet **Règles**, la section **Vos modifications des règles des médicaments et termes médicaux** permet de :

- **Essayer les règles** : tapez une phrase pour voir ce que les règles en font.
- **Trouver une règle intégrée** (avec **Afficher les règles intégrées**) : cherchez par le mot correct (« Takadol ») ou par ce que la transcription a écrit (« tacadol »), puis modifiez le texte reconnu ou le remplacement, ou désactivez la règle.
- **Ajouter une règle** : un texte mal reconnu (expression régulière, sans distinction de casse, limites de mot ajoutées automatiquement) et son remplacement. Les règles ajoutées passent avant les règles intégrées.
- La liste en tête de section montre tout ce que vous avez changé. Chaque règle peut être rétablie, et le bouton de réinitialisation revient exactement aux règles intégrées.

Ces modifications sont enregistrées à part (`medical_rules.json`), séparément de vos règles personnalisées : importer ou réinitialiser l'un ne touche pas l'autre. Pour corriger autre chose qu'un médicament (un nom propre, par exemple), utilisez plutôt les règles personnalisées.

## Contexte pour les dictées courtes

Activé par défaut : **Paramètres > Système > Avancé > Contexte pour les dictées courtes**.

Une dictée très courte, par exemple un seul mot que l'on redicte pour le corriger, se reconnaît mal toute seule : le modèle n'a ni le timbre de votre voix ni la phrase autour. Quand une dictée dure moins de 2 secondes ou donne 3 mots au plus, Plumed la retranscrit derrière les 10 dernières secondes de la dictée précédente, séparées par 2 secondes de silence, puis ne garde que les mots prononcés après ce silence (repérés par leur horodatage, pas en comparant les textes).

Cet audio reste uniquement en mémoire vive, n'est jamais écrit sur le disque, et est oublié 5 minutes après la dernière dictée, à la fermeture de Plumed, ou dès que l'option est désactivée.

## PhoneMic (dicter depuis son téléphone, alpha)

> **Alpha :** cette fonctionnalité est une préversion, attendez-vous à des bugs.

Désactivé par défaut : **PhoneMic** dans la barre latérale.

Avec PhoneMic, le téléphone sert de micro, même s'il n'est pas sur le même réseau que l'ordinateur (4G, Wi-Fi invité de l'hôpital) :

1. Cliquer sur **Connecter un téléphone** : un QR code s'affiche.
2. Le scanner avec l'appareil photo du téléphone. La page PhoneMic de Plumed s'ouvre (en français, ou en anglais), sans rien installer.
3. Les deux écrans affichent un code (par exemple `524C-F8E8-D5A4-2D82`). S'il est identique, confirmer sur l'ordinateur.
4. Dicter sur le téléphone : le texte transcrit est collé dans l'application active de l'ordinateur, comme pour une dictée normale. En mode mains libres, le téléphone écoute en continu et n'envoie que la parole, jamais les silences.

L'audio passe par un serveur relais (`https://turn.plumed.fr` par défaut, modifiable), mais il est chiffré de bout en bout entre le téléphone et Plumed : le relais ne peut pas l'écouter. La vérification du code empêche un relais malveillant de s'intercaler. Cette garantie suppose que le relais sert une page téléphone non modifiée (la page et son code de chiffrement viennent du relais) : n'utilisez qu'un relais de confiance, ou hébergez le vôtre. Le relais voit en revanche les adresses IP et la taille des paquets chiffrés. Le téléphone se reconnecte tout seul après une coupure réseau, mais recharger la page demande de scanner à nouveau le QR code. Une fois connecté, le bouton « Appairer un autre téléphone » réaffiche le QR code pour changer de téléphone.

Seul le relais HTTPS est utilisé (pas de WebRTC, souvent bloqué sur les réseaux hospitaliers), et le proxy système est respecté.

## Règles de dictée InterHop (installées par défaut)

L'association [InterHop](https://interhop.org/) maintient des règles de dictée médicale pour Murmure (unités, constantes, ponctuation dictée, modèles d'examen déclenchés par « ajoute la commande ... ») : [framagit.org/interhop/murmure-regex](https://framagit.org/interhop/murmure-regex). Par exemple « tension 120 80 » devient « TA : 120/80 » et « 3 fois par jour » devient « 3/j ».

Une nouvelle installation de Plumed les contient déjà dans **Paramètres > Corrections > Règles** (section **Vos règles de formatage**), comme règles personnalisées ordinaires : vous pouvez les modifier, les désactiver ou les supprimer, et ajouter les vôtres. Elles s'appliquent après la correction des médicaments.

Sur une installation existante (ou après les avoir supprimées), vous pouvez les importer :

1. Télécharger [`config.murmure`](https://framagit.org/interhop/murmure-regex/-/raw/main/config.murmure).
2. Dans **Paramètres > Importer / Exporter**, importer ce fichier avec la stratégie **Fusionner**, pour garder vos propres règles (**Remplacer** les supprime). Les règles InterHop déjà présentes sont mises à jour, pas dupliquées.

En ligne de commande : `plumed import config.murmure --strategy merge`. Sur un poste verrouillé, l'option **Paramètres > Système > Avancé > Bloquer l'import en ligne de commande** refuse cette commande.

Attention : l'import, quelle que soit la stratégie, applique aussi les options intégrées d'InterHop (espace avant ? et !, conversion des nombres en chiffres en français, etc.). La correction des médicaments reste activée.

Rédigé avec Claude Code.
