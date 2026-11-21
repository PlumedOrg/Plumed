# Bundled French medical data (Plumed fork)

## regex-fixes/

Drug-name and medical-term fix rules (25,099 regexes, mined from the errors of the UltiMed v2 model), applied by `src-tauri/src/medical/drug_rules.rs` in this order: `drug_fix_rules.jsonl` (2,447 rules), `dictionary_fix_rules.jsonl` (18,261), `parhaf_fix_rules.jsonl` (2,172), `parrot_fix_rules.jsonl` (117), `acronyms_fix_rules.jsonl` (43), then `manual_fix_rules.jsonl` (2,059, hand-written, last so its spelling conventions also respell what a learned rule wrote).

- Byte for byte copies of the `rules/` folder of https://github.com/UltiMed-Org/UltiMed-ASR-FR-v2-regex-fixes (plus its `confusion_groups.jsonl`, which is not a rule file: the decoder's second pass, `src-tauri/src/engine/confusion.rs`, re-scores on the audio the decoded words listed there, keeping only the about 140 genuinely ambiguous groups of the 905). The commit and SHA-256 of each file are in `regex-fixes/PROVENANCE`. Do not edit them here: users change rules from the app (their edits are stored on top, see below), and rule improvements go to the rules repo.
- How the rules are built: https://github.com/UltiMed-Org/UltiMed-ASR-FR-v2-regex-fixes/tree/main/scripts
- License: AGPL-3.0-or-later, like the rules repo. Drug names come from French open data (Licence Ouverte / Etalab 2.0: OPEN_MEDIC, RETROCEDAM, BDPM); medical terms come from public French sources (terms only, no definitions). See the rules repo README for the attribution.

User edits made in the app (disabled, changed or added rules) are stored in `medical_rules.json` in the app data folder, separate from the user's formatting rules. Deleting them ("Reset") goes back to exactly these files.

### Refresh

```bash
# from a local clone of the rules repo (default: ../../../ASR_parakeet_finetuning_ultimed_nemo/UltiMed-ASR-FR-v2-regex-fixes)
./sync-regex-fixes.sh [path/to/UltiMed-ASR-FR-v2-regex-fixes]
```

Then check that the Rust port still gives exactly ParakeetWeb's output (`app/ui/src/lib/drugRules.js`, which compiles the same JSONL files):

```bash
# in ParakeetWeb: one {"in", "out"} JSON line per text of a corpus, through compileDrugRuleSource + applyDrugRules
# then, in src-tauri/:
PLUMED_DRUG_PARITY=/path/to/parity.jsonl cargo test --release --lib medical -- --ignored
```

That also checks that every rule still parses and compiles.

## interhop/

InterHop's French medical dictation rules (138 regexes: units, vital signs, dictated punctuation, exam templates triggered by "ajoute la commande ..."), seeded as the user's formatting rules on a fresh install by `src-tauri/src/medical/interhop.rs`. From then on they are ordinary user rules: editable, removable, exported and imported like any other. Existing installs keep their rules; they can import InterHop's `config.murmure` with "Fusionner" (same rule ids, so nothing is duplicated, but that import also overwrites the built-in options).

- Source: https://framagit.org/interhop/murmure-regex (`regex.csv`). The commit is in `interhop/PROVENANCE`.
- `rules.json` is generated from `regex.csv` by InterHop's own converter (`convert_csv_to_murmure.py`, loaded from the clone): same parsing, same skipped rows, same ids as their `config.murmure`. Each rule keeps its example, and the Rust tests check that every example gives the expected text, alone and with all the rules together.
- License: AGPL-3.0 (the rules were written by the Plumed maintainer, who added the license file to the repository).

### Refresh

```bash
# from a local clone of murmure-regex (default: ../../../parakeet_web/framagit_murmure)
python sync-interhop-rules.py [path/to/murmure-regex]
cd .. && cargo test --lib interhop
```

## phrase-boosting/

French medical phrase boosting list (~77,000 phrases: drugs, medical terms, LoRN bacteria names, dosage units), loaded into the decoder at model load by `src-tauri/src/medical/phrase_list.rs`. During decoding it nudges the model toward these phrases (ParakeetWeb's trie engine, `src-tauri/src/engine/phrase_boost/`), on top of the user dictionary.

- Copy of ParakeetWeb's `phrase_boosting/french_medical.txt` (https://github.com/thiswillbeyourgithub/parakeet_web) with only its first line, the defaults line, replaced by Plumed's tuning (`#!commitment 0` and min-p 0.01, chosen by the benchmark in `AGENTS.FORK.md`). The source commit and SHA-256 are in `phrase-boosting/PROVENANCE`. Do not edit it here: list improvements go to ParakeetWeb.
- License: AGPL-3.0, like ParakeetWeb, except the bacterial names merged in from LPSN (List of Prokaryotic names with Standing in Nomenclature, https://lpsn.dsmz.de/), which stay under CC BY-SA 4.0 (see ParakeetWeb's `ATTRIBUTION.md`). Required attribution: Parte, A.C., Sardà Carbasse, J., Meier-Kolthoff, J.P., Reimer, L.C. & Göker, M. (2020). List of Prokaryotic names with Standing in Nomenclature (LPSN) moves to the DSMZ. International Journal of Systematic and Evolutionary Microbiology 70, 5607-5612. https://doi.org/10.1099/ijsem.0.004332

### Refresh

```bash
# from a local clone of ParakeetWeb (default: ../../../parakeet_web)
./sync-phrase-list.sh [path/to/parakeet_web]
cd .. && cargo test --lib phrase_list
```

Set up with Claude Code.
