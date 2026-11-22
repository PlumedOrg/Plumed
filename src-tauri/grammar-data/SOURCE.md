# Grammar step data (Plumed fork)

Written with Claude Code.

## Grammalecte

- Grammalecte 2.3.0, French grammar checker by Olivier R., https://grammalecte.net, GNU GPL v3 or later (compatible with Plumed's AGPL v3). License text: `LICENSE-grammalecte.txt` (the `LICENSE.txt` of the official 2.3.0 archive), shipped next to the code in `resources/grammalecte/LICENSE.txt`.
- What Plumed ships: the JavaScript build of Grammalecte's Firefox extension, unmodified, version 2.3.0: https://addons.mozilla.org/firefox/downloads/file/4643560/grammalecte_fr-2.3.0.xpi (SHA-256 `443030004dbf62ecd5fc42f7b96872b96c9e3765f1cba4704a2a4170bb945256`, checked by `scripts/fetch-grammalecte.sh`). Only the files listed in `grammalecte-files.txt` are kept.
- Source offer: the shipped JavaScript is the unminified source; the full source archive of the same version is https://grammalecte.net/zip/Grammalecte-fr-v2.3.0.zip and the source repository is http://grammalecte.net:8080/.
- To move to a new version: update the URL and hash in `scripts/fetch-grammalecte.sh`, check the file list, run the parity test (`grammar::engine_tests::parity_with_the_python_reference`, local only), and re-measure the rules.

## grammar_rules.json

Built from the UltiMed evaluation bundle (Grammalecte 2.3.0 on the UltiMed v2 model's transcriptions after the six v2 regex files): `grammalecte_allowlist.json` (the 8 default rules, `skip_types`, the 18 masks verbatim, in Python `re` syntax; `case_sensitive` is true for the masks without `IGNORECASE`) and `eval/al_final/families_full.tsv` / `rule_ids_full.tsv` (per family and per rule: single-suggestion fixes with the masks on the dev set, how many made the transcription closer to or further from the reference). Aggregate counts only: no dictation text. The default rules come first, then the other families by precision.
