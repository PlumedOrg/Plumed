# Plumed 0.2.0-beta.3 (2026-11-23)

Everything in 0.2.0-beta.2, plus:
- **Dictation rules**: the 45 UltiMed dictation rules (units, vital signs, dictated punctuation and layout, including "nouvel alinéa") are read by Plumed itself, no longer through murmure-regex, and are now called UltiMed rules instead of InterHop. The "examen infectieux" rule is gone. Existing installs keep their rules and edits.
- **Medical rules**: UltiMed rules v2.1.7, also published as medical data update 2026.11.23.4.
- **Text templates**: the bundled templates can be read and edited, and they come after the grammar correction (step 8).
- **Tutorial**: examples turn from purple to grey once dictated, the ready-made notes show only the bundled "examen clinique normal", and the finish button goes away once clicked.
- **Interface**: taller Corrections tabs with arrow buttons and click and drag, line breaks shown as a return sign in the overlay and recent dictations, "par Olivier Cornelis" below the sidebar icon, the Murmure section of About moved below the Plumed one.
- **Model**: no Download or Check button any more: onboarding continues the download and the model is verified automatically.
