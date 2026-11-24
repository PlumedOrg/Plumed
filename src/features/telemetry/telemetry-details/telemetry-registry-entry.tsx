import { ReactNode } from 'react';
import { useTranslation } from '@/i18n';
import privacyPolicy from '../../../../PRIVACY_POLICY.md?raw';

// Plumed fork (written with Claude Code): the usage statistics entry of the
// "Registre des traitements", read from PRIVACY_POLICY.md (the only copy) so
// the opt-in shows exactly what the published policy says.
//
// Expected shape in the policy, for example:
//   ### Statistiques d'utilisation anonymes (metrics.plumed.fr)
//
//   - **Finalité** : mesurer l'usage de l'application...
//   - **Base légale** : consentement...
// plus the policy's "Responsable du traitement : ..." line, shown below them.
// Renamed heading: the section is hidden and an error is logged.
const HEADING = "### Statistiques d'utilisation anonymes (metrics.plumed.fr)";

/** The bullet lines under `heading`, up to the next heading. */
const sectionItems = (markdown: string, heading: string): string[] | null => {
    const lines = markdown.split('\n');
    const start = lines.findIndex((line) => line.trim() === heading);
    if (start === -1) return null;
    const items: string[] = [];
    for (const line of lines.slice(start + 1)) {
        if (line.startsWith('#')) break;
        if (line.startsWith('- ')) items.push(line.slice(2));
    }
    return items;
};

/** `**bold**` becomes <strong>, backticks are dropped: the only markup used in the entry. */
const inline = (text: string): ReactNode[] =>
    text
        .replaceAll('`', '')
        .split(/\*\*(.+?)\*\*/)
        .map((part, i) => (i % 2 === 1 ? <strong key={i}>{part}</strong> : part));

const CONTROLLER = privacyPolicy.split('\n').find((line) => line.startsWith('Responsable du traitement'));
const ITEMS = sectionItems(privacyPolicy, HEADING);
if (ITEMS === null || ITEMS.length === 0) {
    console.error(`PRIVACY_POLICY.md has no entry under "${HEADING}"`);
}

export const TelemetryRegistryEntry = () => {
    const { t } = useTranslation();
    if (ITEMS === null || ITEMS.length === 0) return null;
    return (
        <details data-testid="telemetry-registry-entry">
            <summary className="cursor-pointer text-foreground">{t('Record of processing activities (GDPR)')}</summary>
            <ul className="mt-2 list-disc space-y-1 pl-5">
                {ITEMS.map((item) => (
                    <li key={item}>{inline(item)}</li>
                ))}
            </ul>
            {CONTROLLER !== undefined && <p className="mt-2">{inline(CONTROLLER)}</p>}
        </details>
    );
};
