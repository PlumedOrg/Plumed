import { ExternalLink } from '@/components/external-link';
import { useTranslation } from '@/i18n';

// Plumed fork credits (written with Claude Code). The text lives here in
// both languages rather than in fr.json, so upstream locale files stay
// untouched and the Plumed renaming of translated strings cannot alter
// the sentences that name the original Murmure.

const PARAKEET_WEB = 'https://github.com/thiswillbeyourgithub/parakeet_web';
// The fix rules credited below come from the rules repo, not the model repo.
const ULTIMED_RULES = 'https://github.com/UltiMed-Org/UltiMed-ASR-FR-v2-regex-fixes';
const LPSN = 'https://lpsn.dsmz.de/';
const GRAMMALECTE = 'https://grammalecte.net';
// Source offer for the bundled Grammalecte build (GPL v3): the pinned extension package
// (the JavaScript Plumed ships, unminified) and the upstream source archive of the same version.
const GRAMMALECTE_PACKAGE = 'https://addons.mozilla.org/firefox/downloads/file/4643560/grammalecte_fr-2.3.0.xpi';
const GRAMMALECTE_SOURCE = 'https://grammalecte.net/zip/Grammalecte-fr-v2.3.0.zip';

interface CreditsText {
    intro: [string, string, string];
    dataTitle: string;
    drugRules: [string, string];
    dictationRules: string;
    phraseList: [string, string];
    grammar: [string, string, string, string];
    upstream: string;
}

const TEXT: Record<'fr' | 'en', CreditsText> = {
    en: {
        intro: [
            'Plumed is a fork of Murmure for French medical dictation, made by ',
            ', psychiatrist and developer. It adds the medical mode of ',
            '.',
        ],
        dataTitle: 'Bundled medical rules:',
        drugRules: [
            'Drug and medical term fix rules: ',
            ' (AGPL-3.0-or-later, built on French open drug data, Etalab 2.0)',
        ],
        dictationRules: 'Medical dictation rules: ',
        phraseList: ['Medical phrase boosting list: ', ' (AGPL v3). Bacterial names from '],
        grammar: [
            'Grammar checker (Corrections step 7): ',
            ' 2.3.0 by Olivier R. (GNU GPL v3 or later), shipped unmodified with its license. Source: ',
            'the bundled package',
            'the source archive',
        ],
        upstream: 'Murmure is made by Al1x-ai. Please support the original project below.',
    },
    fr: {
        intro: [
            'Plumed est un fork de Murmure dédié à la dictée médicale en français, réalisé par ',
            ', psychiatre et développeur. Il reprend le mode médical de ',
            '.',
        ],
        dataTitle: 'Règles médicales incluses :',
        drugRules: [
            'Règles de correction des médicaments et termes médicaux : ',
            " (AGPL-3.0-or-later, d'après les données ouvertes françaises sur le médicament, Etalab 2.0)",
        ],
        dictationRules: 'Règles de dictée médicale : ',
        phraseList: ["Liste d'amplification de phrases médicales : ", ' (AGPL v3). Noms de bactéries issus de '],
        grammar: [
            'Correcteur grammatical (étape 7 des corrections) : ',
            ' 2.3.0 par Olivier R. (GNU GPL v3 ou ultérieure), inclus sans modification avec sa licence. Source : ',
            'le paquet inclus',
            "l'archive des sources",
        ],
        upstream: 'Murmure est développé par Al1x-ai. Soutenez le projet original ci-dessous.',
    },
};

export const ForkCredits = () => {
    const { i18n } = useTranslation();
    const lang = i18n.language.startsWith('fr') ? 'fr' : 'en';
    const text = TEXT[lang];

    return (
        <section className="w-full space-y-3 rounded-2xl border-2 border-border/60 bg-background/50 p-5 text-left text-sm text-muted-foreground">
            <h2 className="text-center text-xl font-semibold tracking-tight text-foreground">Plumed</h2>
            <p>
                {text.intro[0]}
                <ExternalLink href={`https://olicorne.org/${lang}`} withIcon={false}>
                    Olivier Cornelis
                </ExternalLink>
                {text.intro[1]}
                <ExternalLink href={PARAKEET_WEB} withIcon={false}>
                    ParakeetWeb
                </ExternalLink>
                {text.intro[2]}
            </p>
            <div>
                <p>{text.dataTitle}</p>
                <ul className="list-disc space-y-1 pl-5">
                    <li>
                        {text.drugRules[0]}
                        <ExternalLink href={ULTIMED_RULES} withIcon={false}>
                            UltiMed
                        </ExternalLink>
                        {text.drugRules[1]}
                    </li>
                    <li>
                        {text.dictationRules}
                        <ExternalLink href={ULTIMED_RULES} withIcon={false}>
                            UltiMed
                        </ExternalLink>{' '}
                        (AGPL-3.0-or-later)
                    </li>
                    <li>
                        {text.phraseList[0]}
                        <ExternalLink href={PARAKEET_WEB} withIcon={false}>
                            ParakeetWeb
                        </ExternalLink>
                        {text.phraseList[1]}
                        <ExternalLink href={LPSN} withIcon={false}>
                            LPSN
                        </ExternalLink>{' '}
                        (CC BY-SA 4.0)
                    </li>
                    <li>
                        {text.grammar[0]}
                        <ExternalLink href={GRAMMALECTE} withIcon={false}>
                            Grammalecte
                        </ExternalLink>
                        {text.grammar[1]}
                        <ExternalLink href={GRAMMALECTE_PACKAGE} withIcon={false}>
                            {text.grammar[2]}
                        </ExternalLink>
                        {', '}
                        <ExternalLink href={GRAMMALECTE_SOURCE} withIcon={false}>
                            {text.grammar[3]}
                        </ExternalLink>
                        .
                    </li>
                </ul>
            </div>
            <p className="text-xs">{text.upstream}</p>
        </section>
    );
};
