import { useTranslation } from '@/i18n';
import { MedicalCategoryKey, MedicalChanges } from '../medical-changes/medical-changes';

// Plumed fork (written with Claude Code): read-only list of what an imported
// medical list would change, so a shared file cannot slip in a rule or a text
// template unseen. Each list imports as a whole (no per-entry choice).

interface ChangeLine {
    /** 'added' also covers edited model corrections. */
    kind: 'added' | 'disabled';
    text: string;
}

const overlayLines = <T,>(
    overlay: { disabled: string[]; added: T[] } | undefined,
    show: (entry: T) => string
): ChangeLine[] => [
    ...(overlay?.added ?? []).map((entry) => ({ kind: 'added' as const, text: show(entry) })),
    ...(overlay?.disabled ?? []).map((key) => ({ kind: 'disabled' as const, text: key })),
];

/**
 * Example: changeLines('text_templates', { text_templates: { added: [{ trigger: 'normal', text: 'RAS' }], disabled: [] } })
 * gives [{ kind: 'added', text: 'normal → RAS' }].
 */
export const changeLines = (key: MedicalCategoryKey, changes: MedicalChanges): ChangeLine[] => {
    switch (key) {
        case 'medical_phrases':
            return overlayLines(changes.medical_phrases, (phrase) => phrase);
        case 'medical_confusion_groups':
            return overlayLines(
                changes.medical_confusion_groups,
                (group) => `${group.heard} → ${group.candidates.join(', ')}`
            );
        case 'grammar_rules':
            return overlayLines(changes.grammar_rules, (id) => id);
        case 'text_templates':
            return overlayLines(changes.text_templates, (template) => `${template.trigger} → ${template.text}`);
        case 'medical_rules':
            return (changes.medical_rules ?? []).map((rule) => ({
                kind: rule.enabled ? 'added' : 'disabled',
                text: `${rule.core} → ${rule.replacement}`,
            }));
    }
};

interface MedicalChangesReviewProps {
    lines: ChangeLine[];
}

export const MedicalChangesReview = ({ lines }: MedicalChangesReviewProps) => {
    const { t } = useTranslation();
    return (
        <ul className="space-y-1 py-1 text-sm max-h-64 overflow-y-auto">
            {lines.map((line, index) => (
                <li key={index} className="flex gap-2 text-muted-foreground">
                    <span className="shrink-0 w-24 text-xs">
                        {line.kind === 'added' ? t('Added or changed') : t('Turned off')}
                    </span>
                    <span className="whitespace-pre-wrap break-all font-mono text-xs text-foreground">{line.text}</span>
                </li>
            ))}
        </ul>
    );
};
