import { useState } from 'react';
import { useLocation, useNavigate } from '@tanstack/react-router';
import { Page } from '@/components/page';
import { Tabs } from '@/components/tabs';
import { Typography } from '@/components/typography';
import { useTranslation } from '@/i18n';
import { CustomDictionary } from '@/features/personalize/custom-dictionary/custom-dictionary';
import { CorrectionsExplanation } from './corrections-explanation/corrections-explanation';
import { CleanUpTab } from './clean-up-tab/clean-up-tab';
import {
    CORRECTIONS_PATH,
    CorrectionsTab,
    CorrectionsTarget,
    numberedTabLabel,
    tabFromHash,
} from './corrections.helpers';
import { FormattingTab } from './formatting-tab/formatting-tab';
import { MedicalVocabulary } from './medical-vocabulary/medical-vocabulary';
import { ModelCorrections } from './model-corrections/model-corrections';
import { SoundAlikeWords } from './sound-alike-words/sound-alike-words';

// Plumed fork (written with Claude Code): one page for everything that changes
// the transcribed text, one tab per list. The URL hash selects the tab, so
// links can open one.
export const Corrections = () => {
    const { t } = useTranslation();
    const { hash } = useLocation();
    const navigate = useNavigate();
    const activeTab = tabFromHash(hash);
    const [anchor, setAnchor] = useState<string | null>(null);

    const goToTab = (tab: CorrectionsTab) => void navigate({ to: CORRECTIONS_PATH, hash: tab, replace: true });

    const selectTab = (tab: CorrectionsTab) => {
        setAnchor(null);
        goToTab(tab);
    };

    const open = (target: CorrectionsTarget) => {
        setAnchor(target.anchor ?? null);
        goToTab(target.tab);
    };

    return (
        <main className="space-y-6">
            <Page.Header>
                <Typography.MainTitle data-testid="corrections-title">{t('Corrections')}</Typography.MainTitle>
            </Page.Header>

            <Tabs
                value={activeTab}
                onValueChange={selectTab}
                tabs={[
                    { value: 'explanation', label: t('Explanation') },
                    { value: 'vocabulary', label: numberedTabLabel('vocabulary', t('Medical vocabulary')) },
                    { value: 'sound-alikes', label: numberedTabLabel('sound-alikes', t('Sound-alike words')) },
                    { value: 'dictionary', label: numberedTabLabel('dictionary', t('Personal dictionary')) },
                    { value: 'clean-up', label: numberedTabLabel('clean-up', t('Clean-up and standardization')) },
                    {
                        value: 'model-corrections',
                        label: numberedTabLabel('model-corrections', t('Model corrections')),
                    },
                    { value: 'formatting', label: numberedTabLabel('formatting', t('Formatting rules')) },
                ]}
            >
                {activeTab === 'explanation' && <CorrectionsExplanation onOpen={open} />}
                {activeTab === 'vocabulary' && <MedicalVocabulary />}
                {activeTab === 'sound-alikes' && <SoundAlikeWords />}
                {activeTab === 'dictionary' && <CustomDictionary />}
                {activeTab === 'clean-up' && <CleanUpTab anchor={anchor} />}
                {activeTab === 'model-corrections' && <ModelCorrections />}
                {activeTab === 'formatting' && <FormattingTab />}
            </Tabs>
        </main>
    );
};
