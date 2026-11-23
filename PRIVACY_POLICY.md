# Politique de confidentialité (Plumed)

**Dernière mise à jour :** 23 novembre 2026

Plumed est une application de bureau de reconnaissance vocale libre, qui place la confidentialité au premier plan. Elle est conçue pour fonctionner sans connexion internet et pour garder vos données vocales sur votre appareil.

## Résumé

- Rien n'est envoyé, sauf si vous acceptez les **statistiques d'utilisation anonymes** (désactivées par défaut, voir plus bas).
- Votre audio et vos transcriptions sont traités **localement** sur votre appareil.
- Plumed ne conserve que ce dont il a besoin pour fonctionner (réglages, mots et règles personnalisés). Aucune transcription n'est écrite sur le disque par défaut.
- Au premier lancement, Plumed télécharge le modèle de reconnaissance vocale que vous choisissez, depuis Hugging Face ou, si Hugging Face est inaccessible, depuis le relais de modèles de Plumed (`models.plumed.fr`). Il s'agit d'un simple téléchargement de fichiers, sans identifiant ; il a lieu une seule fois et le modèle est conservé lors des mises à jour.
- Au démarrage, Plumed vérifie les mises à jour en téléchargeant un petit fichier depuis GitHub, ou depuis le relais de Plumed (`models.plumed.fr`) si GitHub est inaccessible ; aucun identifiant n'est envoyé. Vous pouvez choisir les versions stables ou bêta, ou désactiver la vérification automatique, dans Paramètres > En ligne. Le téléchargement des mises à jour est soumis à votre accord.
- Au premier lancement, Plumed demande s'il peut vérifier, à chaque démarrage, les mises à jour des règles médicales. Uniquement si vous acceptez (vous pouvez le modifier dans Paramètres > En ligne), il demande à GitHub, au démarrage, s'il existe de nouvelles règles médicales : un simple téléchargement d'un petit fichier signé, sans identifiant. Rien n'est contacté avant que vous ayez répondu. Les nouvelles règles ne sont téléchargées que lorsque vous cliquez sur « Télécharger ». Vous pouvez aussi les télécharger vous-même avec votre navigateur et les installer avec « Installer une mise à jour téléchargée », ce qui n'effectue aucune requête réseau.

## Ce que Plumed ne fait pas

- Il ne vend aucune donnée d'utilisateur et n'en collecte aucune, sauf si vous acceptez les statistiques d'utilisation anonymes.
- Il n'envoie pas vos enregistrements ni vos transcriptions vers un service cloud par défaut.
- Il n'envoie jamais ce que vous dictez, votre audio ou un quelconque texte, même lorsque les statistiques d'utilisation sont activées.

## Données stockées sur votre appareil

Plumed stocke localement les éléments suivants dans le dossier de données de l'application :

- **Les réglages** (raccourcis, paramètres d'affichage, langue)
- **Les mots du dictionnaire personnalisé**
- **Les règles de mise en forme**
- **Les règles médicales** (`medical_rules.json`, vos modifications des corrections des médicaments et des termes médicaux)
- **Les modifications du vocabulaire médical** (`medical_phrases.json`, les expressions que vous avez désactivées ou ajoutées à la liste d'expressions médicales)
- **Les modifications des mots qui se ressemblent** (`medical_confusion_groups.json`, les groupes que vous avez désactivés ou ajoutés)
- **Les modifications des règles de grammaire** (`grammar_rules.json`, les règles de grammaire que vous avez désactivées ou activées)
- **Les modifications des modèles de texte** (`text_templates.json`, les modèles de texte que vous avez ajoutés, modifiés, activés ou désactivés)
- **Les journaux de l'application** (ils ne contiennent pas de transcriptions par défaut)
- **Les compteurs des statistiques d'utilisation** (`telemetry.json`, uniquement si vous les avez acceptées, voir plus bas)
- **Les règles médicales téléchargées** (dossier `medical-data/`, uniquement si vous avez choisi de télécharger une mise à jour des données médicales ; aucune donnée personnelle)
- **Les modèles de reconnaissance vocale** (dossier `models/`, les fichiers de modèle que vous avez téléchargés et celui qui est utilisé ; aucune donnée personnelle)

Par défaut, **aucune transcription n'est conservée sur le disque**. L'historique des transcriptions est conservé uniquement en mémoire et il est effacé à la fermeture de l'application. Si vous activez explicitement l'option « Persistance de l'historique », les 5 dernières transcriptions sont enregistrées sur le disque, dans le dossier de données de l'application. Chaque entrée de l'historique conserve aussi le texte obtenu après chaque étape de traitement (pour montrer sur la page d'accueil ce que chaque étape de correction a modifié) ; ces textes suivent exactement les mêmes règles : en mémoire uniquement par défaut, sur le disque uniquement avec « Persistance de l'historique », et uniquement pour ces 5 dernières transcriptions.

Ces données locales ne sont pas transmises par défaut.

## Étape de grammaire

Le correcteur grammatical (Grammalecte, étape 7 des corrections) est intégré à Plumed et s'exécute sur votre ordinateur. Le texte qu'il vérifie n'est jamais envoyé nulle part et cette étape ne le conserve pas.

## Microphone

Plumed n'écoute pas votre microphone en dehors d'une session d'enregistrement active.

Pendant un enregistrement, l'audio est écrit dans un fichier WAV temporaire, dans un dossier `plumed_recordings` que seul votre compte utilisateur peut lire : sous Linux, dans le dossier d'exécution de la session (`$XDG_RUNTIME_DIR`, en mémoire, effacé à la déconnexion) lorsqu'il y en a un d'utilisable, sinon dans le dossier de cache de Plumed. Le fichier est supprimé après la transcription, sauf si vous avez activé « Conserver les enregistrements audio », qui conserve les 5 derniers.

## Contexte pour les dictées courtes

Activé par défaut. Pour mieux reconnaître les dictées très courtes (moins de 2 secondes, ou 3 mots au plus), Plumed conserve les 10 dernières secondes de l'audio des dictées récentes **en mémoire uniquement** et transcrit la dictée courte à la suite de cet audio. Cet audio n'est jamais écrit sur le disque, ne quitte jamais votre ordinateur et est supprimé 5 minutes après la dernière dictée, à la fermeture de Plumed, ou lorsque l'option est désactivée.

## PhoneMic (alpha)

Cette fonctionnalité est une préversion alpha : attendez-vous à des bogues.

PhoneMic (le microphone distant) permet à un téléphone, sur n'importe quel réseau, de servir de microphone, par l'intermédiaire d'un serveur relais (par défaut `https://turn.plumed.fr`, configurable). Il est **désactivé par défaut**, et rien n'est envoyé tant que vous n'avez pas cliqué sur « Connecter un téléphone ».

- La page du téléphone et le relais sont propres à Plumed (code source dans `remote_microphone_server/`). L'audio est **chiffré de bout en bout** (ECDH P-256 et AES-256-GCM) entre le téléphone et Plumed : le relais transmet des paquets qu'il ne peut pas déchiffrer. Cela suppose que le relais serve une page de téléphone non modifiée : la page et son code de chiffrement proviennent du relais, donc un relais compromis pourrait servir une page qui divulgue l'audio.
- Avant que tout audio soit accepté, les deux écrans affichent le même court code et vous devez confirmer sur l'ordinateur qu'il correspond. Cela empêche un relais de s'interposer.
- Le relais conserve tout en mémoire et n'enregistre aucune adresse IP ni aucun identifiant de salon, mais il voit tout de même des métadonnées réseau : les adresses IP, l'horaire et la taille des paquets chiffrés (donc, approximativement, la durée de chaque dictée). N'utilisez qu'un relais auquel vous-même ou votre établissement faites confiance.
- L'audio est conservé en mémoire jusqu'à sa transcription, puis supprimé. La session prend fin lorsque vous cliquez sur « Déconnecter » ou fermez Plumed.

## Statistiques d'utilisation anonymes (sur consentement)

Plumed demande une fois, au démarrage, si vous souhaitez partager des statistiques d'utilisation anonymes. Elles sont **désactivées sauf si vous répondez oui**, et vous pouvez modifier votre réponse à tout moment dans Paramètres > En ligne. Le bouton « Voir ce qui serait envoyé » affiche les données exactes qui seraient envoyées.

- **Finalité** : améliorer Plumed (savoir quelles versions, quels systèmes et quelles fonctionnalités sont utilisés, et à quelle fréquence des erreurs surviennent). Rien d'autre : ni publicité, ni profilage, ni revente.
- **Base légale** : votre consentement (RGPD, art. 6, paragraphe 1, point a).
- **Ce qui est envoyé**, au plus une fois par jour et uniquement les jours où vous utilisez Plumed : la version de l'application, le canal de mise à jour (stable, bêta ou désactivé), le système d'exploitation et l'architecture du processeur, le type d'installateur, la langue de l'interface, le nombre de jours depuis l'activation des statistiques, le nom du modèle (la variante du catalogue utilisée, par exemple `ultra-int8`), le nombre de dictées de la période avec la moyenne et la dispersion de leur durée et de leur longueur en caractères, des tranches de durée (moins de 10 s, de 10 à 30 s, de 30 à 120 s, plus de 120 s), la vitesse de traitement, le nombre d'erreurs (chargement du modèle, microphone, collage), et les fonctionnalités activées (sous forme de oui/non ou de fourchettes grossières comme « 1-10 règles »), y compris le nombre de modèles de texte activés (une fourchette, `text_templates` : 0, 1-10, 11-50 ou plus), et si vous êtes allé au bout du tutoriel intégré (`tutorial_completed`, oui/non).
- **Ce qui n'est jamais envoyé** : ce que vous dictez, l'audio, vos règles ou les mots de votre dictionnaire, les chemins de fichiers, les noms d'ordinateur ou d'utilisateur, l'heure de chaque dictée, le détail du matériel.
- **Identifiant** : un identifiant aléatoire, sans lien avec vous ni avec votre ordinateur, permet de distinguer les jours d'une même installation. Il est renouvelé tous les 13 mois et jamais prolongé.
- **Où et combien de temps** : envoyées à une instance [Umami](https://umami.is) auto-hébergée, exploitée par le mainteneur de Plumed dans l'Union européenne, et conservées au plus 25 mois. Umami ne stocke pas les adresses IP ; toute localisation qu'il déduit de la connexion n'est pas plus fine que le pays ou la ville. Le serveur web placé devant conserve un journal d'accès technique (date, adresse demandée, adresse IP, identification du navigateur) à des fins de dépannage et de sécurité, supprimé automatiquement après 24 heures (base légale : intérêt légitime à maintenir le service sûr et en état de fonctionner).
- **Sur votre ordinateur** : les compteurs de la période en cours et l'identifiant sont conservés dans `telemetry.json`, dans le dossier de données de l'application, jusqu'à leur envoi.
- **Si vous répondez « Plus tard »** : Plumed conserve sur votre ordinateur, dans `telemetry_ask.json`, le nombre de fois où vous avez refusé, la date, ainsi que le nombre de lancements et de dictées depuis (de simples compteurs, jamais envoyés), uniquement pour reposer la question au bout d'au moins 7 jours, 10 dictées et 3 lancements. La troisième fois, « Ne plus jamais demander » est proposé. Toute réponse définitive supprime ce fichier.
- **Retrait du consentement** : désactiver l'option dans les réglages interrompt tout envoi et supprime immédiatement `telemetry.json` (identifiant et compteurs).

Les versions compilées sans serveur de statistiques configuré (par exemple vos propres compilations) n'affichent jamais la question et n'envoient jamais rien.

## Import et export des réglages

Plumed permet d'exporter et d'importer les réglages sous forme de fichiers `.murmure`. Ces fichiers peuvent contenir : les réglages système, les raccourcis, les règles de mise en forme et les mots du dictionnaire.

## Utilisation du réseau

**Ce programme ne transférera aucune information vers d'autres systèmes en réseau, sauf demande expresse de l'utilisateur ou de la personne qui l'installe ou l'exploite.**

Lorsque vous utilisez des fonctionnalités liées au réseau, des métadonnées réseau courantes (comme votre adresse IP) sont nécessairement visibles du service que vous contactez.

Plumed peut accéder au réseau uniquement pour :

- **Télécharger le modèle de reconnaissance vocale** (uniquement lorsque vous cliquez sur « Télécharger » dans la fenêtre du modèle ou dans Paramètres > Modèle) : les fichiers du modèle sont récupérés depuis Hugging Face, ou à défaut depuis le relais de modèles de Plumed (`models.plumed.fr`, exploité par le mainteneur de Plumed, qui relaie les mêmes fichiers depuis Hugging Face et n'enregistre ni les adresses IP ni les fichiers téléchargés). Aucun identifiant, cookie ou donnée d'utilisation n'est envoyé. Les fichiers sont vérifiés à l'aide d'empreintes SHA-256 intégrées à l'application.
- **Vérifier les mises à jour** (s'exécute automatiquement au lancement, sauf si vous le désactivez dans Paramètres > En ligne)
- **Télécharger et installer les mises à jour** (uniquement lorsque vous choisissez d'installer une mise à jour)
- **PhoneMic, le microphone distant** (uniquement après que vous l'avez activé et cliqué sur « Connecter un téléphone » ; voir plus haut)
- **Les statistiques d'utilisation anonymes** (uniquement si vous les avez acceptées ; voir plus haut)
- **Vérifier les mises à jour des données médicales** (au lancement, depuis GitHub, uniquement après que vous l'avez accepté au premier lancement ou avec « Rechercher les mises à jour des règles médicales au démarrage » ; aucun identifiant, cookie ou donnée d'utilisation n'est envoyé) et **les télécharger** (uniquement lorsque vous cliquez sur « Télécharger »)

Les vérifications de mises à jour n'incluent ni votre audio ni vos transcriptions.

Politiques de confidentialité des tiers : [Déclaration de confidentialité de GitHub](https://docs.github.com/en/site-policy/privacy-policies/github-privacy-statement), [Politique de confidentialité de Hugging Face](https://huggingface.co/privacy)

## Accès au presse-papiers

Pour insérer les transcriptions dans d'autres applications, Plumed peut :

- écrire temporairement la transcription dans le presse-papiers du système,
- simuler une action de collage,
- restaurer le contenu précédent du presse-papiers, sauf si vous avez activé une option pour conserver la transcription dans le presse-papiers.

Le contenu du presse-papiers n'est pas transmis par Plumed.

## Pratiques de sécurité

Nous utilisons des outils d'analyse statique tels que **SonarQube** pour aider à détecter d'éventuelles vulnérabilités et problèmes de sécurité. L'analyse automatisée ne couvre pas 100 % du code ni tous les scénarios, et elle ne constitue pas une garantie de sécurité.

## Logiciel libre

Plumed est publié sous la licence GNU AGPL v3. Le code source peut être audité par tous :

[https://github.com/PlumedOrg/Plumed](https://github.com/PlumedOrg/Plumed)

Plumed est un fork de [Murmure](https://www.murmure.app/) (code source : [Kieirra/murmure](https://github.com/Kieirra/murmure)), dont la politique de confidentialité sert de base à ce document.

## Exclusion de garantie (mentions légales)

**LE LOGICIEL EST FOURNI « TEL QUEL », SANS GARANTIE D'AUCUNE SORTE, EXPRESSE OU IMPLICITE.**

Dans toute la mesure permise par la loi, les auteurs et contributeurs ne sauraient être tenus responsables de toute réclamation, de tout dommage ou de toute autre responsabilité découlant du logiciel ou de son utilisation, ou s'y rapportant.
