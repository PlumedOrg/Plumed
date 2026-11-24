# Murmure 1.8.1 (upstream) (2026-04-04)

**🐛 Bug Fixes**

- **(macOS) Fix shortcuts not working** - Space and other shortcuts were not detected on macOS. Replaced rdev event listener with polling-based approach.
- Remove Ctrl+B shortcut that could hide sidebar ([#266](https://github.com/Kieirra/murmure/issues/266)).
- Allow virtual microphones in device list, e.g. Nvidia Broadcast ([#164](https://github.com/Kieirra/murmure/issues/164)).
- Add beta tag and CPU usage warning for Voice Mode.
- Security updates for dependencies.
