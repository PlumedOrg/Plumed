// plumed.fr: the documentation site, in French, built to static files
// (deployed by author_scripts/deploy.sh --only-website). No analytics, no
// external fonts or scripts: everything is served from plumed.fr.
import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';

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
            customCss: ['./src/styles/plumed.css'],
            lastUpdated: false,
            sidebar: [
                { label: 'Installation', slug: 'installation' },
                {
                    label: 'Fonctionnalités',
                    items: [{ autogenerate: { directory: 'fonctionnalites' } }],
                },
                {
                    label: 'Aide',
                    items: [
                        { label: 'Questions fréquentes', slug: 'faq' },
                        { label: 'Dépannage', slug: 'depannage' },
                    ],
                },
                {
                    label: 'À propos',
                    items: [
                        { label: 'Confidentialité', link: '/confidentialite/' },
                        { label: 'Remerciements et licences', slug: 'licences' },
                    ],
                },
            ],
        }),
    ],
});
