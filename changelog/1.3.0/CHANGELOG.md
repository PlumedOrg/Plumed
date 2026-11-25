# Plumed 1.3.0 (2026-11-25)

- **Escape cancels a recording** by default (Paramètres > Raccourcis, still configurable). Installs still on the old default `Ctrl+Backspace` move to Escape, a shortcut you chose yourself is kept. A cancel shortcut made only of keys of the record shortcut (record `Shift+Escape`, cancel `Escape`) would cancel every recording it starts, so it is removed, with a message.
- **French formatting by default** (new installs, Paramètres > Corrections > Nettoyage): numbers from 2 up are written in digits (« deux comprimés » becomes « 2 comprimés », « un » stays a word), and a space goes before ? and !.
- **Update checks** (when enabled) run at startup and then once a day while the app stays open, not only at startup.
- **Updates published one platform at a time**: when a new version is not built yet for your system, the app now says there is no update instead of showing an error.
- **macOS Intel**: the app no longer crashes at launch (its bundled ONNX Runtime was refused).
