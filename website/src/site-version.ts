// The version of plumed.fr: the number of commits that touched the site
// (website/ and PRIVACY_POLICY.md, which the privacy page renders), so each
// such commit bumps it, with the date of the last one. build.sh computes
// both on the VPS (its Node container has no git) and passes them as
// PLUMED_WEBSITE_VERSION and PLUMED_WEBSITE_DATE; a local build asks git.
import { execFileSync } from 'node:child_process';

const PATHS = ['website', 'PRIVACY_POLICY.md'];

function git(...args: string[]): string | undefined {
    try {
        return execFileSync('git', ['-C', '..', ...args], { encoding: 'utf8' }).trim() || undefined;
    } catch {
        return undefined;
    }
}

export const siteVersion =
    process.env.PLUMED_WEBSITE_VERSION || git('rev-list', '--count', 'HEAD', '--', ...PATHS) || 'inconnue';

/** YYYY-MM-DD, or undefined when unknown. */
const isoDate = process.env.PLUMED_WEBSITE_DATE || git('log', '-1', '--format=%cs', '--', ...PATHS);

export const siteDate = isoDate ? new Date(`${isoDate}T12:00:00Z`).toLocaleDateString('fr-FR', { dateStyle: 'long', timeZone: 'UTC' }) : undefined;
