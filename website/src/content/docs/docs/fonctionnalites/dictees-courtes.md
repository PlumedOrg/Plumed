---
title: Contexte pour les dictées courtes
description: Mieux reconnaître un mot isolé grâce à la fin de la dictée précédente.
sidebar:
  order: 5
---

Activé par défaut : **Paramètres > Avancé > Contexte pour les dictées courtes**.

Une dictée très courte, par exemple un seul mot que l'on redicte pour le corriger, se reconnaît mal toute seule : le modèle n'a ni le timbre de votre voix ni la phrase autour. Quand une dictée dure moins de 2 secondes ou donne 3 mots au plus, Plumed la retranscrit derrière les 10 dernières secondes de la dictée précédente, séparées par 2 secondes de silence, puis ne garde que les mots prononcés après ce silence (repérés par leur horodatage, pas en comparant les textes).

Cet audio reste uniquement en mémoire vive, n'est jamais écrit sur le disque, et est oublié 5 minutes après la dernière dictée, à la fermeture de Plumed, ou dès que l'option est désactivée.
