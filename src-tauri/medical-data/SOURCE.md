# Bundled French medical data (Plumed fork)

## regex-fixes/

Drug-name and medical-term fix rules (45,112 regexes, release v2.1.0, mined from the errors of the UltiMed v2 model), applied by `src-tauri/src/medical/drug_rules.rs` in this order: `drug_fix_rules.jsonl` (4,145 rules), `dictionary_fix_rules.jsonl` (36,177), `parhaf_fix_rules.jsonl` (2,502), `parrot_fix_rules.jsonl` (138), `acronyms_fix_rules.jsonl` (47), then `manual_fix_rules.jsonl` (2,103, hand-written, last so its spelling conventions also respell what a learned rule wrote).

- Byte for byte copies of the `rules/` folder of https://github.com/UltiMed-Org/UltiMed-ASR-FR-v2-regex-fixes (plus its `confusion_groups.jsonl`, which is not a rule file: the decoder's second pass, `src-tauri/src/engine/confusion.rs`, re-scores on the audio the decoded words listed there, keeping only the 68 genuinely ambiguous groups of the 415 (v2.0.1 dropped the candidates that only differ by a silent plural or feminine ending)). The commit and SHA-256 of each file are in `regex-fixes/PROVENANCE`. Do not edit them here: users change rules from the app (their edits are stored on top, see below), and rule improvements go to the rules repo.
- How the rules are built: https://github.com/UltiMed-Org/UltiMed-ASR-FR-v2-regex-fixes/tree/main/scripts
- License: AGPL-3.0-or-later, like the rules repo. Drug names come from French open data (Licence Ouverte / Etalab 2.0: OPEN_MEDIC, RETROCEDAM, BDPM); medical terms come from public French sources (terms only, no definitions). See the rules repo README for the attribution.

## plumed-rules/

`plumed_fix_rules.jsonl`: Plumed's own fix rules, for fixes that should not (or not yet) go to the UltiMed rules repo. Same JSONL shape as UltiMed's `manual_fix_rules.jsonl` (`pattern`, `replacement`, `variant`), applied after every UltiMed file. Empty for now. The test `every_plumed_rule_compiles` checks each line. ParakeetWeb does not have these rules: once the file is not empty, the parity check against ParakeetWeb (below) differs on texts they change.

User edits made in the app (disabled, changed or added rules) are stored in `medical_rules.json` in the app data folder, separate from the user's formatting rules. Deleting them ("Back to the original list") goes back to exactly these files. The phrase list and the confusion groups get the same overlay (disabled entries by key, added entries) in `medical_phrases.json` and `medical_confusion_groups.json` (`src-tauri/src/medical/list_overlay.rs`, written with Claude Code).

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

## regex-fixes/dictation_rules.csv

French medical dictation rules (44 regexes: units, vital signs, dictated punctuation and layout), the dictation list `rules/dictation_rules.csv` of [UltiMed-ASR-FR-v2-regex-fixes](https://github.com/UltiMed-Org/UltiMed-ASR-FR-v2-regex-fixes), copied with the fix rules by `sync-regex-fixes.sh` (same tag, same SHA256SUMS check, same `regex-fixes/PROVENANCE`). It is not part of the published medical data: `src-tauri/src/medical/dictation_rules.rs` reads it at build time and seeds the rules as the user's formatting rules on a fresh install. From then on they are ordinary user rules: editable, removable, exported and imported like any other. Existing installs keep their rules.

- Format: `id,remplacement,regex,example_pre,example_post`, blank and `#` lines ignored, `\n` is a newline. Plumed skips the rows its formatter cannot run: lookarounds, and a replacement ending in `\u` (uppercase the next letter, used by the `nouvel alinéa` rule).
- Rule ids are a uuid5 of the examples, hashed exactly as the murmure-regex converter these rules used to go through did (name `regex.csv`), so they did not change when that dependency was dropped. Each rule keeps its example, and the Rust tests check that every example gives the expected text, alone and with all the rules together.
- License: AGPL-3.0 (the rules were written by the Plumed maintainer).

## phrase-boosting/

French medical phrase boosting list (~77,000 phrases: drugs, medical terms, LoRN bacteria names, dosage units), loaded into the decoder at model load by `src-tauri/src/medical/phrase_list.rs`. During decoding it nudges the model toward these phrases (ParakeetWeb's trie engine, `src-tauri/src/engine/phrase_boost/`), on top of the user dictionary.

- Copy of ParakeetWeb's `phrase_boosting/french_medical.txt` (https://github.com/thiswillbeyourgithub/parakeet_web) with only its first line, the defaults line, replaced by Plumed's tuning (`#!commitment 0.5`, weight 0.5, min-p 0.01, re-tuned on the v2 model by the benchmark in `AGENTS.FORK.md`). The source commit and SHA-256 are in `phrase-boosting/PROVENANCE`. Do not edit it here: list improvements go to ParakeetWeb.
- License: AGPL-3.0, like ParakeetWeb, except the bacterial names merged in from LPSN (List of Prokaryotic names with Standing in Nomenclature, https://lpsn.dsmz.de/), which stay under CC BY-SA 4.0 (see ParakeetWeb's `ATTRIBUTION.md`). Required attribution: Parte, A.C., Sardà Carbasse, J., Meier-Kolthoff, J.P., Reimer, L.C. & Göker, M. (2020). List of Prokaryotic names with Standing in Nomenclature (LPSN) moves to the DSMZ. International Journal of Systematic and Evolutionary Microbiology 70, 5607-5612. https://doi.org/10.1099/ijsem.0.004332

### Refresh

```bash
# from a local clone of ParakeetWeb (default: ../../../parakeet_web)
./sync-phrase-list.sh [path/to/parakeet_web]
cd .. && cargo test --lib phrase_list
```

## Online updates (signed)

The rule files, the confusion groups and the phrase list can be updated without an app release (code in `src-tauri/src/medical/data_set/` and `src-tauri/src/medical/data_update.rs`, section written with Claude Code). The bundled files above stay the fallback.

Version of the bundled data: `BUNDLED_VERSION` in `src-tauri/src/medical/data_set/mod.rs` (bump it when the bundled files change, so a downloaded set older than the bundled one is ignored). Model the rules are for: `MODEL_ID` in the same file (`ultimed-v2`, the model the rules were mined from; the default v1 model uses the same rules).

### Release layout

Everything is an asset of the GitHub release with the fixed tag `medical-data` on `PlumedOrg/Plumed`, a prerelease that must never be the "latest" release (the app updater reads `releases/latest/download/latest.json`):

- `https://github.com/PlumedOrg/Plumed/releases/download/medical-data/manifest.json`
- `https://github.com/PlumedOrg/Plumed/releases/download/medical-data/manifest.json.minisig`: its minisign signature
- each file of the manifest, same URL prefix + file name

### manifest.json

```json
{
  "version": "2026.11.21",
  "model": "ultimed-v2",
  "min_app_version": "0.1.4",
  "notes": "Texte court en français",
  "files": [{ "name": "drug_fix_rules.jsonl", "sha256": "<64 lowercase hex>", "size": 123 }]
}
```

- `version`: `YYYY.MM.DD` or `YYYY.MM.DD.N`, compared numerically (`2026.11.21.10` is newer than `2026.11.21.9`). Only a version newer than the one in use is offered.
- `model`: must equal the app's `MODEL_ID`, otherwise the whole manifest is refused (the rules fix the errors of one model).
- `min_app_version`: semver; refused when above the app version.
- `notes`: shown in the update dialog (2,000 characters at most).
- `files`: only these names are accepted, any other name refuses the whole manifest: the seven rule files (`drug_fix_rules.jsonl`, `dictionary_fix_rules.jsonl`, `parhaf_fix_rules.jsonl`, `parrot_fix_rules.jsonl`, `acronyms_fix_rules.jsonl`, `manual_fix_rules.jsonl`, `plumed_fix_rules.jsonl`, all required), `confusion_groups.jsonl` and `french_medical.txt` (optional: missing means the bundled one is kept). Each file is at most 50 MB.

### What the app checks

1. The minisign signature of `manifest.json`, against the public key compiled into the app (`PLUMED_RULES_PUBKEY` at build time). Builds without it never check, download or show anything.
2. The manifest content as above.
3. Each downloaded file's size and SHA-256, before anything is moved into place (download into a staging folder, then a rename).
4. That the new set parses (at most 5% unusable rule lines, a non-empty phrase list and confusion groups when present). Otherwise it is deleted and the previous data stays.

The same checks and install code apply when the user installs a set by hand ("Install a downloaded update"): the release assets downloaded with a browser (`manifest.json`, `manifest.json.minisig` and the data files, under their published names) are verified and copied into a staging folder, then installed like a download. The startup check itself only runs once the user agreed to it on the first launch question.

Installed sets live in `medical-data/<version>/` in the app data folder (files + signed manifest), and `medical-data/active.json` names the one in use. At every start the signature and hashes are checked again; a missing, corrupt or unparsable set, or one not newer than the bundled data, falls back to the bundled data with a warning in the log. The user's changes (`medical_rules.json`, `medical_phrases.json`, `medical_confusion_groups.json`) are never touched; changes to entries that no longer exist are kept and listed in their tab.

### Publishing

`scripts/publish-medical-data.sh <version> "<notes>"` builds the manifest from the files currently in this folder, signs it and uploads everything (see the comment at its top, including how to create the key pair).

Set up with Claude Code.

## text-templates/templates.jsonl

Bundled text templates (Corrections > Text templates tab, written with Claude Code): one JSON object per line with a spoken `trigger` and the `text` it expands to (line breaks as `\n`). The bundled example, a normal clinical examination, is written by hand, system by system, and is turned off by default (the user turns it on in the tab). Not part of the online data updates: it ships with the app.
