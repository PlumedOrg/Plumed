// plumed.fr: the documentation site, in French, built to static files
// (deployed by author_scripts/deploy.sh --only-website). No external fonts
// or scripts: everything is served from plumed.fr, except the Umami script
// when the build gets ANALYTICS_* in its environment (see env.example).
import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';

/** The Umami tag, or nothing when ANALYTICS_URL or ANALYTICS_WEBSITE_ID is unset. */
function analyticsHead() {
    const { ANALYTICS_URL, ANALYTICS_WEBSITE_ID, ANALYTICS_DNT, ANALYTICS_SRI } = process.env;
    if (!ANALYTICS_URL || !ANALYTICS_WEBSITE_ID) {
        return [];
    }
    const attrs = { defer: true, src: ANALYTICS_URL, 'data-website-id': ANALYTICS_WEBSITE_ID };
    if (['1', 'true', 'yes'].includes((ANALYTICS_DNT ?? '').toLowerCase())) {
        attrs['data-do-not-track'] = 'true';
    }
    if (ANALYTICS_SRI) {
        attrs.integrity = ANALYTICS_SRI;
        attrs.crossorigin = 'anonymous';
    }
    return [{ tag: 'script', attrs }];
}

// Search engines: PLUMED is also a well known molecular dynamics library
// (plumed.org), so "Plumed" alone ranks it first. These tags tie this site to
// the dictation app ("Plumed dictée", "Plumed speech-to-text", "Plumed STT")
// and to its GitHub and Hugging Face pages, which link back here.
const STRUCTURED_DATA = {
    '@context': 'https://schema.org',
    '@type': 'SoftwareApplication',
    name: 'Plumed',
    alternateName: ['Plumed dictée vocale', 'Plumed speech-to-text', 'Plumed STT', 'Plumed audio'],
    description:
        'Plumed: free, offline French medical voice dictation (speech-to-text, STT) for Windows, macOS and Linux. Dictée vocale médicale en français, gratuite et hors ligne.',
    url: 'https://plumed.fr',
    image: 'https://plumed.fr/accueil.png',
    applicationCategory: 'HealthApplication',
    operatingSystem: 'Windows, macOS, Linux',
    inLanguage: 'fr',
    isAccessibleForFree: true,
    license: 'https://www.gnu.org/licenses/agpl-3.0.html',
    offers: { '@type': 'Offer', price: '0', priceCurrency: 'EUR' },
    author: { '@type': 'Person', name: 'Olivier Cornelis', url: 'https://olicorne.org/fr' },
    sameAs: ['https://github.com/PlumedOrg/Plumed', 'https://huggingface.co/datasets/Olicorne/UltiMed-ASR-FR-v1'],
};

/** Tags on every page: the preview image of shared links and the structured data. */
const SEO_HEAD = [
    { tag: 'meta', attrs: { property: 'og:image', content: 'https://plumed.fr/accueil.png' } },
    { tag: 'meta', attrs: { name: 'twitter:image', content: 'https://plumed.fr/accueil.png' } },
    { tag: 'script', attrs: { type: 'application/ld+json' }, content: JSON.stringify(STRUCTURED_DATA) },
];

export default defineConfig({
    site: 'https://plumed.fr',
    integrations: [
        starlight({
            title: 'Plumed',
            description:
                'Plumed, dictée vocale médicale en français (speech-to-text) : gratuite, hors ligne et respectueuse de la confidentialité.',
            logo: { src: './src/assets/logo.svg' },
            favicon: '/favicon.svg',
            locales: { root: { label: 'Français', lang: 'fr' } },
            social: [
                {
                    icon: 'github',
                    label: 'Code source sur GitHub',
                    href: 'https://github.com/PlumedOrg/Plumed',
                },
            ],
            head: [...SEO_HEAD, ...analyticsHead()],
            customCss: ['./src/styles/plumed.css'],
            components: {
                // The "Documentation" link next to the GitHub icon
                SocialIcons: './src/components/SocialIcons.astro',
                // The "fresh paint" warning on the /docs pages
                MarkdownContent: './src/components/MarkdownContent.astro',
                // The site version under each page
                Footer: './src/components/Footer.astro',
            },
            lastUpdated: false,
            sidebar: [
                { label: 'Présentation', slug: 'docs' },
                { label: 'Installation', slug: 'docs/installation' },
                {
                    label: 'Fonctionnalités',
                    items: [{ autogenerate: { directory: 'docs/fonctionnalites' } }],
                },
                {
                    label: 'Aide',
                    items: [
                        { label: 'Questions fréquentes', slug: 'docs/faq' },
                        { label: 'Dépannage', slug: 'docs/depannage' },
                    ],
                },
                {
                    label: 'À propos',
                    items: [
                        { label: 'Confidentialité', link: '/confidentialite/' },
                        { label: 'Remerciements et licences', slug: 'docs/licences' },
                    ],
                },
            ],
        }),
    ],
});
