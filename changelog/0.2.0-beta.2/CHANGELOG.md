# Plumed 0.2.0-beta.2 (2026-11-23)

First published 0.2.0 beta (0.2.0-beta.1 was never released: its macOS build found no runner). Everything listed under 0.2.0-beta.1 is included, plus:
- **UltiMed v1 models**: UltiMed v1 Ultra int8 and w4a8 can be downloaded again, and v1 int8 is now the default model (w4a8 on machines with less than 8 GB of RAM). The UltiMed v2 models stay available at the bottom of the list, labelled "(EXPERIMENTAL)": picking one shows a warning that v2 often does much worse than v1 for now. Existing installs keep their model. The medical data (rules, confusion groups, phrase boosting) applies to every model.
- **Medical rules**: UltiMed rules v2.0.3 (45,112 regexes), also published as the first medical data update (2026.11.23.2).
- **macOS**: now requires macOS 13.4 or later (the ONNX Runtime 1.31 build is made for it), and links clang's runtime library, which the macOS build failed without.
- **Build fixes**: Grammalecte packaging on Windows, Arch package version for pre-releases.
