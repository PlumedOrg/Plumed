import { useState } from 'react';
import { Button } from '@/components/button';
import { Input } from '@/components/input';
import { useTranslation } from '@/i18n';
import { useFormattingRules } from '@/features/personalize/formatting-rules/hooks/use-formatting-rules';
import { ConfusionGroup } from '../corrections.types';
import { useListFile } from '../hooks/use-list-file';
import { useListOverlay } from '../hooks/use-list-overlay';
import { ListHeader } from '../list-header/list-header';
import { OverlayList } from '../overlay-list/overlay-list';

const splitCandidates = (text: string): string[] =>
    text
        .split(',')
        .map((candidate) => candidate.trim())
        .filter((candidate) => candidate.length > 0);

const GroupEntry = ({ group }: { group: ConfusionGroup }) => (
    <span>
        <span className="font-mono text-xs">{group.heard}</span>
        {group.candidates.length > 0 && (
            <span className="text-muted-foreground"> → {group.candidates.join(' / ')}</span>
        )}
    </span>
);

/** "Sound-alike words" tab: the confusion groups of the second pass, searchable, with the user's changes. */
export const SoundAlikeWords = () => {
    const { t } = useTranslation();
    const { settings, updateBuiltInOption } = useFormattingRules();
    const { overlay, saveOverlay, reload, changeCount } = useListOverlay<ConfusionGroup>(
        'get_confusion_overlay',
        'set_confusion_overlay'
    );
    const { exportList, importList } = useListFile('confusion_groups', reload);
    const [heard, setHeard] = useState('');
    const [candidates, setCandidates] = useState('');

    const addGroup = async () => {
        const group = { heard: heard.trim(), candidates: splitCandidates(candidates) };
        const saved = await saveOverlay({ ...overlay, added: [...overlay.added, group] });
        if (saved) {
            setHeard('');
            setCandidates('');
        }
    };

    return (
        <div className="space-y-6">
            <ListHeader
                title={t('Tell apart words that sound alike')}
                description={t(
                    'When the model writes one of these forms, Plumed listens to the passage again and keeps the candidate the audio supports best. The forms are the ones the model really writes, so they can look odd. Final transcriptions only.'
                )}
                option={{
                    checked: settings.built_in.confusion_rescoring,
                    onChange: (checked) => updateBuiltInOption('confusion_rescoring', checked),
                    testId: 'option-confusion-rescoring',
                }}
                summary={changeCount}
                resetDescription={t(
                    'Your added groups are deleted and every group of the list is turned back on. This cannot be undone.'
                )}
                onReset={() => saveOverlay({ disabled: [], added: [] })}
                onExport={exportList}
                onImport={importList}
                testId="sound-alike-words"
            />
            <OverlayList
                searchCommand="search_confusion_groups"
                overlay={overlay}
                saveOverlay={saveOverlay}
                renderEntry={(group) => <GroupEntry group={group} />}
                searchPlaceholder={t('Search a word, e.g. rituximab')}
                title={t('Groups')}
                testId="confusion-groups"
            >
                <div className="flex items-center gap-2">
                    <Input
                        value={heard}
                        onChange={(e) => setHeard(e.target.value)}
                        placeholder={t('What the model writes')}
                        className="font-mono text-xs"
                        data-testid="confusion-groups-new-heard"
                    />
                    <span className="text-muted-foreground">→</span>
                    <Input
                        value={candidates}
                        onChange={(e) => setCandidates(e.target.value)}
                        placeholder={t('Candidates, separated by commas (2 or more)')}
                        data-testid="confusion-groups-new-candidates"
                    />
                    <Button
                        variant="secondary"
                        size="sm"
                        disabled={heard.trim().length === 0 || splitCandidates(candidates).length < 2}
                        onClick={addGroup}
                        data-testid="confusion-groups-add"
                    >
                        {t('Add')}
                    </Button>
                </div>
            </OverlayList>
        </div>
    );
};
