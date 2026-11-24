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

export default defineConfig({
    site: 'https://plumed.fr',
    integrations: [
        starlight({
            title: 'Plumed',
            description:
                'Dictée vocale médicale en français, gratuite, hors ligne et respectueuse de la confidentialité.',
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
            head: analyticsHead(),
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
