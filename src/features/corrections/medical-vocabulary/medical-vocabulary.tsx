import { useState } from 'react';
import { Button } from '@/components/button';
import { Input } from '@/components/input';
import { useTranslation } from '@/i18n';
import { useFormattingRules } from '@/features/personalize/formatting-rules/hooks/use-formatting-rules';
import { useListFile } from '../hooks/use-list-file';
import { useListOverlay } from '../hooks/use-list-overlay';
import { ListHeader } from '../list-header/list-header';
import { OverlayList } from '../overlay-list/overlay-list';

/** "Medical vocabulary" tab: the phrase boosting list, searchable, with the user's changes. */
export const MedicalVocabulary = () => {
    const { t } = useTranslation();
    const { settings, updateBuiltInOption } = useFormattingRules();
    const { overlay, saveOverlay, reload, changeCount } = useListOverlay<string>(
        'get_phrase_overlay',
        'set_phrase_overlay'
    );
    const { exportList, importList } = useListFile('phrases', reload);
    const [newPhrase, setNewPhrase] = useState('');

    const addPhrase = async () => {
        const saved = await saveOverlay({ ...overlay, added: [...overlay.added, newPhrase.trim()] });
        if (saved) {
            setNewPhrase('');
        }
    };

    return (
        <div className="space-y-6">
            <ListHeader
                title={t('Boost medical vocabulary')}
                description={t(
                    'While it recognizes your voice, the model is nudged towards the words and phrases of this list (about 77,000, from French medical sources). Turn off a phrase that gets in the way, or add your own.'
                )}
                option={{
                    checked: settings.built_in.phrase_boosting,
                    onChange: (checked) => updateBuiltInOption('phrase_boosting', checked),
                    testId: 'option-phrase-boosting',
                }}
                summary={changeCount}
                resetDescription={t(
                    'Your added phrases are deleted and every phrase of the list is turned back on. This cannot be undone.'
                )}
                onReset={() => saveOverlay({ disabled: [], added: [] })}
                onExport={exportList}
                onImport={importList}
                testId="medical-vocabulary"
            />
            <OverlayList
                searchCommand="search_phrases"
                overlay={overlay}
                saveOverlay={saveOverlay}
                renderEntry={(phrase) => phrase}
                searchPlaceholder={t('Search a phrase, e.g. microangiopathie')}
                title={t('Phrases')}
                testId="medical-phrases"
            >
                <div className="flex items-center gap-2">
                    <Input
                        value={newPhrase}
                        onChange={(e) => setNewPhrase(e.target.value)}
                        placeholder={t('New phrase')}
                        data-testid="medical-phrases-new"
                    />
                    <Button
                        variant="secondary"
                        size="sm"
                        disabled={newPhrase.trim().length === 0}
                        onClick={addPhrase}
                        data-testid="medical-phrases-add"
                    >
                        {t('Add')}
                    </Button>
                </div>
            </OverlayList>
        </div>
    );
};
