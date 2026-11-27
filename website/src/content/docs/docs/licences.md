---
title: Remerciements et licences
description: D'où vient Plumed, et sous quelles licences.
---

Plumed est initialement un fork de [Murmure](https://github.com/Kieirra/murmure) qui intègre les fonctionnalités du mode médical de [ParakeetWeb](http://parakeetweb.olicorne.org/?mode=med) et les modèles [UltiMed](https://github.com/UltiMed-Org). UltiMed et ParakeetWeb sont du même auteur que Plumed.

Plumed est un logiciel libre sous licence [GNU AGPL v3](https://github.com/PlumedOrg/Plumed/blob/medmain/LICENSE), comme Murmure.

- **[Murmure](https://www.murmure.app/)** par [Al1x-ai](https://www.al1x-ai.com/) : l'application dont Plumed est un fork (AGPL v3, [code source](https://github.com/Kieirra/murmure)). Soutenez le projet original !
- **[ParakeetWeb](https://github.com/thiswillbeyourgithub/parakeet_web)** : dont le mode médical était un précurseur de Plumed (AGPL v3).
- **[UltiMed](https://github.com/UltiMed-Org)** : les modèles de reconnaissance vocale [UltiMed v1](https://huggingface.co/Olicorne/parakeet-tdt-0.6b-v3-ultra-UltiMed-v1-onnx) et [UltiMed v2](https://huggingface.co/Olicorne/parakeet-tdt-0.6b-v3-ultra-UltiMed-v2-onnx) (CC BY 4.0 ; la v1 est le modèle par défaut et recommandé jusqu'à la sortie d'UltiMed v3, la v2 est expérimentale), et ses [règles de correction des médicaments et termes médicaux](https://github.com/UltiMed-Org/UltiMed-ASR-FR-v2-regex-fixes) (AGPL-3.0-or-later), construites à partir des données ouvertes françaises sur le médicament.
- **[Nemotron-3-Diarization](https://huggingface.co/Olicorne/Nemotron-3-Diarization-web-onnx)** (NVIDIA, Streaming Sortformer converti en ONNX) : le modèle qui distingue les intervenants d'une réunion, téléchargé à la première utilisation de la page Réunion (licence OpenMDW-1.1).
- **[UltiMed-ASR-FR-v2-regex-fixes](https://github.com/UltiMed-Org/UltiMed-ASR-FR-v2-regex-fixes)** (liste `dictation_rules.csv`) : règles de dictée médicale (unités, constantes, ponctuation dictée), proposées par défaut comme règles de formatage (AGPL-3.0-or-later).
- **[LPSN](https://lpsn.dsmz.de/)** (List of Prokaryotic names with Standing in Nomenclature) : noms de bactéries inclus dans la liste d'amplification de phrases, sous licence [CC BY-SA 4.0](https://creativecommons.org/licenses/by-sa/4.0/). Citation requise : Parte, A.C., Sardà Carbasse, J., Meier-Kolthoff, J.P., Reimer, L.C. & Göker, M. (2020). List of Prokaryotic names with Standing in Nomenclature (LPSN) moves to the DSMZ. _International Journal of Systematic and Evolutionary Microbiology_ 70, 5607-5612. https://doi.org/10.1099/ijsem.0.004332
- **[Grammalecte](https://grammalecte.net)** par Olivier R. : le correcteur grammatical de l'étape 7 des corrections, version 2.3.0 (GNU GPL v3 ou ultérieure), inclus sans modification (sa version JavaScript, celle de l'extension Firefox) avec sa licence. Sources : [le paquet inclus](https://addons.mozilla.org/firefox/downloads/file/4643560/grammalecte_fr-2.3.0.xpi) et [l'archive des sources de la même version](https://grammalecte.net/zip/Grammalecte-fr-v2.3.0.zip). Détails dans [`grammar-data/SOURCE.md`](https://github.com/PlumedOrg/Plumed/blob/medmain/src-tauri/grammar-data/SOURCE.md).

Merci enfin aux testeurs, dont les retours ont façonné Plumed.

Le détail des règles médicales incluses (provenance, mise à jour) est dans [`medical-data/SOURCE.md`](https://github.com/PlumedOrg/Plumed/blob/medmain/src-tauri/medical-data/SOURCE.md).

Le contenu de ce site est sous la même licence que le code (AGPL v3).
