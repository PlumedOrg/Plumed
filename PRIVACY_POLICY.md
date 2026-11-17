# Privacy Policy (Plumed)

**Last updated:** November 14, 2026

Plumed is a privacy-first, open-source speech-to-text desktop application. It is designed to work without an internet connection and to keep your speech data on your device.

## Summary

- Plumed does **not** include analytics or telemetry.
- Your audio and transcriptions are processed **locally** on your device.
- Plumed stores only what it needs to operate (settings, custom words/rules). No transcription is written to disk by default.
- On startup, Plumed checks for updates by contacting the configured updater endpoint (currently hosted on GitHub). Downloading updates is opt-in.

## What Plumed does not do

- It does not collect or sell user data.
- It does not upload your recordings or transcriptions to a cloud service by default.
- It does not track your usage with analytics/telemetry.

## Data stored on your device

Plumed stores the following locally in the app's data directory:

- **Settings** (shortcuts, display settings, language)
- **Custom dictionary words**
- **Formatting rules**
- **Medical rules** (`medical_rules.json`, your edits to the drug and medical term corrections)
- **Application logs** (do not contain transcriptions by default)

By default, **no transcription is persisted to disk**. Transcription history is kept in memory only and is cleared when the application closes. If you explicitly enable the "Persist history" option, the last 5 transcriptions will be saved on disk in the app's data directory.

This local data is not transmitted by default.

## Voice mode

Voice mode allows Plumed to listen for a wake word to start recording hands-free. This feature is **disabled by default** and must be explicitly enabled by the user.

When enabled, voice mode maintains a continuous audio stream from your microphone to detect speech. Only detected speech segments are transcribed locally to check for wake word matches. The raw audio is **immediately discarded** after processing. All processing happens on-device: no audio or transcription data leaves your computer.

When disabled (default), Plumed does not listen to your microphone outside of an active recording session.

## Context for short dictations

Enabled by default. To recognize very short dictations better (under 2 seconds, or 3 words or fewer), Plumed keeps the last 10 seconds of recent dictation audio **in memory only** and transcribes the short dictation behind it. This audio is never written to disk, never leaves your computer, and is discarded 5 minutes after the last dictation, when Plumed closes, or when the option is turned off.

## Smart Mic

Smart Mic lets a phone on the same local network act as a microphone. It is **disabled by default**. When enabled, audio travels from the phone to this computer over the local network only. If you configure a relay URL, audio transits through that relay server: use only a relay you host yourself for medical dictation.

## Remote microphone (alpha)

This feature is an alpha pre-release: expect bugs.

The remote microphone lets a phone on any network act as a microphone, through a relay server (by default `https://turn.plumed.fr`, configurable). It is **disabled by default**, and nothing is sent until you click "Connect a phone".

- The phone page and the relay are Plumed's own (source in `remote_microphone_server/`). Audio is **encrypted end to end** (ECDH P-256 and AES-256-GCM) between the phone and Plumed: the relay forwards packets it cannot decrypt.
- Before any audio is accepted, both screens show the same short code and you must confirm on the computer that it matches. This blocks a relay that would try to put itself in the middle.
- The relay keeps everything in memory and logs no IP address or room id, but it still sees network metadata: IP addresses, timing and the size of the encrypted packets (so roughly how long each dictation is). Use only a relay you or your institution trust.
- Audio is kept in memory until it is transcribed, then discarded. The session ends when you click "Disconnect" or close Plumed.

## Settings import/export

Plumed allows exporting and importing settings as `.murmure` files. These files may contain: system settings, shortcuts, formatting rules, and dictionary words.

## Network usage

**This program will not transfer any information to other networked systems unless specifically requested by the user or the person installing or operating it.**

When you use network-related features, standard network metadata (such as your IP address) will necessarily be visible to the service you contact.

Plumed may access the network only for:

- **Checking for updates** (runs automatically on launch; currently not disableable in-app)
- **Downloading and installing updates** (only when you choose to install an update)
- **The remote microphone** (only after you enable it and click "Connect a phone"; see above)

The update check does not include your audio or transcriptions.

Third-party policy (GitHub): [GitHub Privacy Statement](https://docs.github.com/en/site-policy/privacy-policies/github-privacy-statement)

## Clipboard access

To insert transcriptions into other applications, Plumed may:

- temporarily write the transcription to the system clipboard,
- simulate a paste action,
- restore the previous clipboard content unless you enabled an option to keep the transcription in the clipboard.

Clipboard content is not transmitted by Plumed.

## Security practices

We use static analysis tools such as **SonarQube** to help detect potential vulnerabilities and security issues. Automated analysis does not cover 100% of the code or all scenarios, and it is not a guarantee of security.

## Open source

Plumed is released under the GNU AGPL v3 License. The source code can be audited by anyone:

[https://github.com/PlumedOrg/Plumed](https://github.com/PlumedOrg/Plumed)

Plumed is a fork of [Murmure](https://www.murmure.app/) (source: [Kieirra/murmure](https://github.com/Kieirra/murmure)), whose privacy policy this document is based on.

## Disclaimer of warranty (legal)

**THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED.**

To the maximum extent permitted by law, the authors and contributors shall not be liable for any claim, damages, or other liability arising from, out of, or in connection with the software or the use of the software.
