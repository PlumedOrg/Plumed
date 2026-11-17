// French (default) and English strings of the phone page. The choice is a
// preference kept in localStorage, it is not user data.

const STORAGE_KEY = 'plumed-lang';

const STRINGS = {
    fr: {
        page_title: 'Plumed : micro distant (alpha)',
        title: 'Micro distant Plumed',
        alpha_notice: 'Version alpha : fonctionnalité en cours de test, des problèmes sont possibles.',
        privacy_notice:
            "L'audio est chiffré de bout en bout entre ce téléphone et votre ordinateur, si ce serveur relais est de confiance.",
        loading: 'Connexion…',
        no_link: 'Ouvrez cette page en scannant le QR code affiché par Plumed sur votre ordinateur.',
        unsupported: 'Ce navigateur ne permet pas le chiffrement nécessaire. Utilisez un navigateur récent.',
        pairing_intro: "Vérifiez que ce code est identique à celui affiché sur l'ordinateur :",
        pairing_wait: "Puis confirmez sur l'ordinateur.",
        denied: "Appairage refusé sur l'ordinateur. Scannez à nouveau le QR code pour réessayer.",
        stale: "Ce QR code n'est plus valide. Scannez le nouveau QR code affiché sur l'ordinateur.",
        replaced:
            'Un autre téléphone a été appairé à la place de celui-ci. Scannez à nouveau le QR code pour le reprendre.',
        closed: "La session est terminée sur l'ordinateur. Scannez le nouveau QR code pour vous reconnecter.",
        mic_error: "Impossible d'accéder au micro : {error}",
        mode_manual: 'Manuel',
        mode_handsfree: 'Mains libres',
        hint_manual: 'Touchez pour démarrer puis pour arrêter, ou maintenez appuyé pour parler.',
        hint_handsfree: "Le micro écoute en continu et n'envoie que lorsque vous parlez.",
        main_start: 'Démarrer',
        main_stop: 'Arrêter',
        main_listen: 'Écouter',
        main_stop_listening: "Arrêter l'écoute",
        state_idle: 'Prêt',
        state_recording: 'Enregistrement…',
        state_listening: 'À l’écoute…',
        state_speech: 'Parole détectée, envoi…',
        state_paused: 'En pause',
        state_sending_file: 'Envoi du fichier…',
        state_received: "Reçu par l'ordinateur",
        pause: 'Pause',
        resume: 'Reprendre',
        send_file: 'Envoyer un fichier audio',
        file_too_long: 'Fichier trop long (20 minutes maximum).',
        file_too_large: 'Fichier trop volumineux (256 Mo maximum).',
        file_error: 'Fichier audio illisible.',
        busy: "Terminez d'abord l'enregistrement en cours.",
        reconnecting: 'Connexion perdue, reconnexion…',
        no_wakelock: "Gardez l'écran allumé : ce navigateur ne peut pas empêcher la mise en veille.",
    },
    en: {
        page_title: 'Plumed: remote microphone (alpha)',
        title: 'Plumed remote microphone',
        alpha_notice: 'Alpha version: this feature is being tested, expect problems.',
        privacy_notice:
            'Audio is end-to-end encrypted between this phone and your computer, provided this relay server is trusted.',
        loading: 'Connecting…',
        no_link: 'Open this page by scanning the QR code shown by Plumed on your computer.',
        unsupported: 'This browser lacks the required encryption. Use a recent browser.',
        pairing_intro: 'Check that this code matches the one shown on the computer:',
        pairing_wait: 'Then confirm on the computer.',
        denied: 'Pairing refused on the computer. Scan the QR code again to retry.',
        stale: 'This QR code is no longer valid. Scan the new QR code shown on the computer.',
        replaced: 'Another phone was paired instead of this one. Scan the QR code again to take over.',
        closed: 'The session ended on the computer. Scan the new QR code to reconnect.',
        mic_error: 'Cannot access the microphone: {error}',
        mode_manual: 'Manual',
        mode_handsfree: 'Hands-free',
        hint_manual: 'Tap to start then to stop, or press and hold to talk.',
        hint_handsfree: 'The microphone listens continuously and only sends when you speak.',
        main_start: 'Start',
        main_stop: 'Stop',
        main_listen: 'Listen',
        main_stop_listening: 'Stop listening',
        state_idle: 'Ready',
        state_recording: 'Recording…',
        state_listening: 'Listening…',
        state_speech: 'Speech detected, sending…',
        state_paused: 'Paused',
        state_sending_file: 'Sending the file…',
        state_received: 'Received by the computer',
        pause: 'Pause',
        resume: 'Resume',
        send_file: 'Send an audio file',
        file_too_long: 'File too long (20 minutes maximum).',
        file_too_large: 'File too large (256 MB maximum).',
        file_error: 'Unreadable audio file.',
        busy: 'Finish the current recording first.',
        reconnecting: 'Connection lost, reconnecting…',
        no_wakelock: 'Keep the screen on: this browser cannot prevent sleep.',
    },
};

export const LANGS = Object.keys(STRINGS);

function stored() {
    const lang = localStorage.getItem(STORAGE_KEY);
    return LANGS.includes(lang) ? lang : 'fr';
}

let lang = stored();

export function currentLang() {
    return lang;
}

export function t(key, vars = {}) {
    const text = STRINGS[lang][key] ?? key;
    return text.replace(/\{(\w+)\}/g, (_, name) => String(vars[name] ?? ''));
}

/** Fills every [data-i18n] element of the page. */
export function applyStatic() {
    document.documentElement.lang = lang;
    document.title = t('page_title');
    for (const el of document.querySelectorAll('[data-i18n]')) el.textContent = t(el.dataset.i18n);
}

export function setLang(next) {
    if (!LANGS.includes(next)) return;
    lang = next;
    localStorage.setItem(STORAGE_KEY, next);
    applyStatic();
}
