# Privacy Policy (Plumed)

**Last updated:** November 21, 2026

Plumed is a privacy-first, open-source speech-to-text desktop application. It is designed to work without an internet connection and to keep your speech data on your device.

## Summary

- Nothing is sent unless you opt in to **anonymous usage statistics** (off by default, see below).
- Your audio and transcriptions are processed **locally** on your device.
- Plumed stores only what it needs to operate (settings, custom words/rules). No transcription is written to disk by default.
- On startup, Plumed checks for updates by contacting the configured updater endpoint (currently hosted on GitHub). Downloading updates is opt-in.

## What Plumed does not do

- It does not sell user data, and collects none unless you opt in to anonymous usage statistics.
- It does not upload your recordings or transcriptions to a cloud service by default.
- It never sends what you dictate, your audio or any patient data, even with usage statistics on.

## Data stored on your device

Plumed stores the following locally in the app's data directory:

- **Settings** (shortcuts, display settings, language)
- **Custom dictionary words**
- **Formatting rules**
- **Medical rules** (`medical_rules.json`, your edits to the drug and medical term corrections)
- **Application logs** (do not contain transcriptions by default)
- **Usage statistics counters** (`telemetry.json`, only if you opted in, see below)

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

- The phone page and the relay are Plumed's own (source in `remote_microphone_server/`). Audio is **encrypted end to end** (ECDH P-256 and AES-256-GCM) between the phone and Plumed: the relay forwards packets it cannot decrypt. This assumes the relay serves an unmodified phone page: the page and its crypto code come from the relay, so a compromised relay could serve a page that leaks audio.
- Before any audio is accepted, both screens show the same short code and you must confirm on the computer that it matches. This blocks a relay that would try to put itself in the middle.
- The relay keeps everything in memory and logs no IP address or room id, but it still sees network metadata: IP addresses, timing and the size of the encrypted packets (so roughly how long each dictation is). Use only a relay you or your institution trust.
- Audio is kept in memory until it is transcribed, then discarded. The session ends when you click "Disconnect" or close Plumed.

## Anonymous usage statistics (opt-in)

Plumed asks once, at startup, whether you want to share anonymous usage statistics. They are **off unless you say yes**, and you can change your answer at any time in Settings > System > Advanced. The "See exactly what is sent" button shows the exact data that would be sent.

- **Purpose**: improve Plumed (know which versions, systems and features are used, and how often errors happen). Nothing else: no advertising, no profiling, no resale.
- **Legal basis**: your consent (GDPR art. 6(1)(a)).
- **What is sent**, at most once a day and only on days you use Plumed: app version, operating system and processor architecture, installer type, interface language, days since you turned the statistics on, the model name, the number of dictations of the period with their average and spread of duration and length in characters, duration buckets (under 10 s, 10 to 30 s, 30 to 120 s, over 120 s), processing speed, error counts (model loading, microphone, paste), and which features are on (as yes/no or coarse ranges such as "1-10 rules").
- **What is never sent**: what you dictate, audio, patient data, your rules or dictionary words, file paths, computer or user names, the time of each dictation, detailed hardware.
- **Identifier**: a random identifier, unrelated to you or your computer, tells the days of the same installation apart. It is renewed every 13 months and never extended.
- **Where and how long**: sent to a self-hosted [Umami](https://umami.is) instance run by the Plumed maintainer in the European Union, kept at most 25 months. Umami does not store IP addresses; any location it derives from the connection is no finer than country or city.
- **On your computer**: the counters of the current period and the identifier are kept in `telemetry.json` in the app's data directory until they are sent.
- **Withdrawal**: turning the option off in the settings stops all sending and deletes `telemetry.json` (identifier and counters) immediately.

Builds made without a statistics server configured (for example your own builds) never show the question and never send anything.

## Settings import/export

Plumed allows exporting and importing settings as `.murmure` files. These files may contain: system settings, shortcuts, formatting rules, and dictionary words.

## Network usage

**This program will not transfer any information to other networked systems unless specifically requested by the user or the person installing or operating it.**

When you use network-related features, standard network metadata (such as your IP address) will necessarily be visible to the service you contact.

Plumed may access the network only for:

- **Checking for updates** (runs automatically on launch; currently not disableable in-app)
- **Downloading and installing updates** (only when you choose to install an update)
- **The remote microphone** (only after you enable it and click "Connect a phone"; see above)
- **Anonymous usage statistics** (only if you opted in; see above)

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
