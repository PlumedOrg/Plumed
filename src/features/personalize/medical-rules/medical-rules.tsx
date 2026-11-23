import { useState } from 'react';
import { Button } from '@/components/button';
import { Input } from '@/components/input';
import { SettingsUI } from '@/components/settings-ui';
import { Textarea } from '@/components/textarea';
import { Typography } from '@/components/typography';
import { useTranslation } from '@/i18n';
import { PAGE_SIZE, Pagination } from '@/features/corrections/pagination/pagination';
import { textArgs, useDebouncedInvoke } from './hooks/use-debounced-invoke';
import { useOrphanedEdits } from './hooks/use-orphaned-edits';
import { MedicalRuleRow } from './medical-rule-row/medical-rule-row';
import { editStatus, findEdit, removeEditAt, replaceEditAt, upsertEdit } from './medical-rules.helpers';
import { MedicalRuleEdit, MedicalRuleView } from './types';

const NO_RESULTS: MedicalRuleView[] = [];

interface MedicalRulesProps {
    edits: MedicalRuleEdit[];
    saveEdits: (edits: MedicalRuleEdit[]) => Promise<boolean>;
}

/** The user's changes to the drug and medical term rules: search of the bundled rules, list, add, try. */
export const MedicalRules = ({ edits, saveEdits }: MedicalRulesProps) => {
    const { t } = useTranslation();
    const [query, setQuery] = useState('');
    const [sample, setSample] = useState('');
    const [newCore, setNewCore] = useState('');
    const [newReplacement, setNewReplacement] = useState('');
    const [offset, setOffset] = useState(0);

    const results = useDebouncedInvoke('search_medical_rules', textArgs('query', query), NO_RESULTS);
    const preview = useDebouncedInvoke('preview_medical_fixes', textArgs('text', sample), '', edits);
    const orphaned = useOrphanedEdits(edits);
    const shownEdits = edits.slice(offset, offset + PAGE_SIZE);

    const addRule = async () => {
        const saved = await saveEdits([
            ...edits,
            { original: null, core: newCore, replacement: newReplacement, enabled: true },
        ]);
        if (saved) {
            setNewCore('');
            setNewReplacement('');
        }
    };

    return (
        <SettingsUI.Section
            title={t('Correction rules')}
            titleTestId="medical-rules-title"
            aside={<span className="text-xs text-muted-foreground">{edits.length}</span>}
        >
            <div className="space-y-6 p-4">
                <section className="space-y-2" data-testid="medical-rules-search">
                    <Typography.Title>{t('Find a pre-installed rule')}</Typography.Title>
                    <Typography.Paragraph>
                        {t('Search by the right word, or by what the transcription wrote.')}
                    </Typography.Paragraph>
                    <Input value={query} onChange={(e) => setQuery(e.target.value)} placeholder="rispéridone" />
                    {results.map((rule) => {
                        const edit = findEdit(edits, rule.original);
                        return (
                            <MedicalRuleRow
                                key={rule.original}
                                rule={edit ?? { ...rule, enabled: true }}
                                status={edit == null ? 'bundled' : 'changed'}
                                onSave={(changed) => saveEdits(upsertEdit(edits, changed))}
                                onRemove={() => saveEdits(edits.filter((other) => other !== edit))}
                            />
                        );
                    })}
                </section>
                <section className="space-y-2">
                    <Typography.Title>{t('Your changes')}</Typography.Title>
                    {edits.length === 0 && (
                        <Typography.Paragraph data-testid="medical-rules-no-changes">
                            {t('No changes yet: the bundled rules apply as they are.')}
                        </Typography.Paragraph>
                    )}
                    {orphaned.size > 0 && (
                        <Typography.Paragraph data-testid="medical-rules-orphaned">
                            {t(
                                '{{count}} of your edits refer to rules that no longer exist in the medical rules in use. They are kept but change nothing.',
                                { count: orphaned.size }
                            )}
                        </Typography.Paragraph>
                    )}
                    {shownEdits.map((edit, shownIndex) => {
                        const index = offset + shownIndex;
                        return (
                            <MedicalRuleRow
                                key={`${edit.original ?? 'new'}-${index}`}
                                rule={edit}
                                status={editStatus(edit, orphaned)}
                                onSave={(changed) => saveEdits(replaceEditAt(edits, index, changed))}
                                onRemove={() => saveEdits(removeEditAt(edits, index))}
                            />
                        );
                    })}
                    <Pagination offset={offset} total={edits.length} onOffsetChange={setOffset} />
                </section>

                <section className="space-y-2">
                    <Typography.Title>{t('Add a rule')}</Typography.Title>
                    <div className="flex items-center gap-2">
                        <Input
                            value={newCore}
                            onChange={(e) => setNewCore(e.target.value)}
                            placeholder={t('Misheard text')}
                            className="font-mono text-xs"
                        />
                        <span className="text-muted-foreground">→</span>
                        <Input
                            value={newReplacement}
                            onChange={(e) => setNewReplacement(e.target.value)}
                            placeholder={t('Replacement')}
                        />
                        <Button
                            variant="secondary"
                            size="sm"
                            disabled={newCore.trim().length === 0 || newReplacement.trim().length === 0}
                            onClick={addRule}
                            data-testid="medical-rules-add"
                        >
                            {t('Add')}
                        </Button>
                    </div>
                </section>

                <section className="space-y-2">
                    <Typography.Title>{t('Try the rules')}</Typography.Title>
                    <Textarea
                        value={sample}
                        onChange={(e) => setSample(e.target.value)}
                        placeholder={t('Type or paste a transcription...')}
                    />
                    {preview.length > 0 && (
                        <p className="text-sm" data-testid="medical-rules-preview">
                            {preview}
                        </p>
                    )}
                </section>
            </div>
        </SettingsUI.Section>
    );
};
