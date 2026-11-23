# Privacy Policy (Plumed)

**Last updated:** November 23, 2026

Plumed is a privacy-first, open-source speech-to-text desktop application. It is designed to work without an internet connection and to keep your speech data on your device.

## Summary

- Nothing is sent unless you opt in to **anonymous usage statistics** (off by default, see below).
- Your audio and transcriptions are processed **locally** on your device.
- Plumed stores only what it needs to operate (settings, custom words/rules). No transcription is written to disk by default.
- On first launch, Plumed downloads the speech model you pick, from Hugging Face or, if Hugging Face cannot be reached, from the Plumed model relay (`models.plumed.fr`). This is a plain file download with no identifier; it happens once and the model is kept across updates.
- On startup, Plumed checks for updates by downloading a small file from GitHub, or from the Plumed relay (`models.plumed.fr`) if GitHub cannot be reached; no identifier is sent. You can choose stable or beta versions, or turn the automatic check off, in Settings > System > Online. Downloading updates is opt-in.
- On first launch, Plumed asks whether it may check for medical rules updates at each start. Only if you agree (you can change it in Settings > System > Online) does it ask GitHub, on startup, whether newer medical rules exist: a plain download of a small signed file, with no identifier. Nothing is contacted before you answer. The new rules are downloaded only when you click "Download". You can also download them yourself with your browser and install them with "Install a downloaded update", which makes no network request.

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
- **Medical vocabulary changes** (`medical_phrases.json`, the phrases you turned off or added to the medical phrase list)
- **Sound-alike word changes** (`medical_confusion_groups.json`, the groups you turned off or added)
- **Grammar rule changes** (`grammar_rules.json`, the grammar rules you turned off or on)
- **Text template changes** (`text_templates.json`, the text templates you added, edited, turned on or off)
- **Application logs** (do not contain transcriptions by default)
- **Usage statistics counters** (`telemetry.json`, only if you opted in, see below)
- **Downloaded medical rules** (`medical-data/` folder, only if you chose to download a medical data update; no personal data)
- **Speech models** (`models/` folder, the model files you downloaded and which one is in use; no personal data)

By default, **no transcription is persisted to disk**. Transcription history is kept in memory only and is cleared when the application closes. If you explicitly enable the "Persist history" option, the last 5 transcriptions will be saved on disk in the app's data directory. Each history entry also keeps the text after each processing step (to show on the home page what each correction step changed); these texts follow exactly the same rules: in memory only by default, on disk only with "Persist history", and only for those last 5 transcriptions.

This local data is not transmitted by default.

## Grammar step

The grammar checker (Grammalecte, step 7 of the corrections) is embedded in Plumed and runs on your computer. The text it checks is never sent anywhere and is not stored by this step.

## Microphone

Plumed does not listen to your microphone outside of an active recording session.

During a recording, the audio is written to a temporary WAV file in a `plumed_recordings` folder that only your user account can read: on Linux in the session runtime folder (`$XDG_RUNTIME_DIR`, in memory, erased at logout) when there is a usable one, otherwise in Plumed's cache folder. The file is deleted after transcription, unless you turned on "Keep audio recordings", which keeps the last 5.

## Context for short dictations

Enabled by default. To recognize very short dictations better (under 2 seconds, or 3 words or fewer), Plumed keeps the last 10 seconds of recent dictation audio **in memory only** and transcribes the short dictation behind it. This audio is never written to disk, never leaves your computer, and is discarded 5 minutes after the last dictation, when Plumed closes, or when the option is turned off.

## PhoneMic (alpha)

This feature is an alpha pre-release: expect bugs.

PhoneMic (the remote microphone) lets a phone on any network act as a microphone, through a relay server (by default `https://turn.plumed.fr`, configurable). It is **disabled by default**, and nothing is sent until you click "Connect a phone".

- The phone page and the relay are Plumed's own (source in `remote_microphone_server/`). Audio is **encrypted end to end** (ECDH P-256 and AES-256-GCM) between the phone and Plumed: the relay forwards packets it cannot decrypt. This assumes the relay serves an unmodified phone page: the page and its crypto code come from the relay, so a compromised relay could serve a page that leaks audio.
- Before any audio is accepted, both screens show the same short code and you must confirm on the computer that it matches. This blocks a relay that would try to put itself in the middle.
- The relay keeps everything in memory and logs no IP address or room id, but it still sees network metadata: IP addresses, timing and the size of the encrypted packets (so roughly how long each dictation is). Use only a relay you or your institution trust.
- Audio is kept in memory until it is transcribed, then discarded. The session ends when you click "Disconnect" or close Plumed.

## Anonymous usage statistics (opt-in)

Plumed asks once, at startup, whether you want to share anonymous usage statistics. They are **off unless you say yes**, and you can change your answer at any time in Settings > System > Online. The "See exactly what is sent" button shows the exact data that would be sent.

- **Purpose**: improve Plumed (know which versions, systems and features are used, and how often errors happen). Nothing else: no advertising, no profiling, no resale.
- **Legal basis**: your consent (GDPR art. 6(1)(a)).
- **What is sent**, at most once a day and only on days you use Plumed: app version, update channel (stable, beta or disabled), operating system and processor architecture, installer type, interface language, days since you turned the statistics on, the model name (which catalog variant is in use, for example `ultra-int8`), the number of dictations of the period with their average and spread of duration and length in characters, duration buckets (under 10 s, 10 to 30 s, 30 to 120 s, over 120 s), processing speed, error counts (model loading, microphone, paste), and which features are on (as yes/no or coarse ranges such as "1-10 rules"), including how many text templates are turned on (a range, `text_templates`: 0, 1-10, 11-50 or more), and whether you ended the built-in tutorial (`tutorial_completed`, yes/no).
- **What is never sent**: what you dictate, audio, patient data, your rules or dictionary words, file paths, computer or user names, the time of each dictation, detailed hardware.
- **Identifier**: a random identifier, unrelated to you or your computer, tells the days of the same installation apart. It is renewed every 13 months and never extended.
- **Where and how long**: sent to a self-hosted [Umami](https://umami.is) instance run by the Plumed maintainer in the European Union, kept at most 25 months. Umami does not store IP addresses; any location it derives from the connection is no finer than country or city. The web server in front of it keeps a technical access log (date, requested address, IP address, browser identification) for troubleshooting and security, deleted automatically after 24 hours (legal basis: legitimate interest in keeping the service secure and working).
- **On your computer**: the counters of the current period and the identifier are kept in `telemetry.json` in the app's data directory until they are sent.
- **If you answer "Later"**: Plumed keeps on your computer, in `telemetry_ask.json`, how many times you declined, the date, and the number of launches and dictations since then (plain counters, never sent), only to ask again after at least 7 days, 10 dictations and 3 launches. The third time, "Never ask again" is offered. Any definitive answer deletes this file.
- **Withdrawal**: turning the option off in the settings stops all sending and deletes `telemetry.json` (identifier and counters) immediately.

Builds made without a statistics server configured (for example your own builds) never show the question and never send anything.

## Settings import/export

Plumed allows exporting and importing settings as `.murmure` files. These files may contain: system settings, shortcuts, formatting rules, and dictionary words.

## Network usage

**This program will not transfer any information to other networked systems unless specifically requested by the user or the person installing or operating it.**

When you use network-related features, standard network metadata (such as your IP address) will necessarily be visible to the service you contact.

Plumed may access the network only for:

- **Downloading the speech model** (only when you click "Download" in the model dialog or in Settings > System > Model): the model files are fetched from Hugging Face, or else from the Plumed model relay (`models.plumed.fr`, run by the Plumed maintainer, which streams the same files from Hugging Face and logs neither IP addresses nor downloaded files). No identifier, cookie or usage data is sent. The files are checked against SHA-256 hashes built into the app.
- **Checking for updates** (runs automatically on launch unless turned off in Settings > System > Online)
- **Downloading and installing updates** (only when you choose to install an update)
- **PhoneMic, the remote microphone** (only after you enable it and click "Connect a phone"; see above)
- **Anonymous usage statistics** (only if you opted in; see above)
- **Checking for medical data updates** (on launch, from GitHub, only after you agreed to it on first launch or with "check for medical data updates at startup"; no identifier, cookie or usage data is sent) and **downloading them** (only when you click "Download")

The update checks do not include your audio or transcriptions.

Third-party policies: [GitHub Privacy Statement](https://docs.github.com/en/site-policy/privacy-policies/github-privacy-statement), [Hugging Face Privacy Policy](https://huggingface.co/privacy)

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
