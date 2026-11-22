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
    CORRECTIONS_TAB_LABELS,
    CORRECTIONS_TABS,
    CorrectionsTab,
    CorrectionsTarget,
    tabFromHash,
    tabStepNumber,
} from './corrections.helpers';
import { StepBadge } from './step-badge/step-badge';
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
                tabs={CORRECTIONS_TABS.map((tab) => {
                    const step = tabStepNumber(tab);
                    return {
                        value: tab,
                        label: t(CORRECTIONS_TAB_LABELS[tab]),
                        prefix: step === null ? undefined : <StepBadge step={step} />,
                    };
                })}
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
