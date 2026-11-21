import { useState } from 'react';
import { useLocation, useNavigate } from '@tanstack/react-router';
import { Page } from '@/components/page';
import { Tabs } from '@/components/tabs';
import { Typography } from '@/components/typography';
import { useTranslation } from '@/i18n';
import { CustomDictionary } from '@/features/personalize/custom-dictionary/custom-dictionary';
import { FormattingRules } from '@/features/personalize/formatting-rules/formatting-rules';
import { CorrectionsExplanation } from './corrections-explanation/corrections-explanation';
import { CORRECTIONS_PATH, CorrectionsTab, CorrectionsTarget, tabFromHash } from './corrections.helpers';
import { useShowBuiltInRules } from './hooks/use-show-built-in-rules';

// Plumed fork (written with Claude Code): one page for everything that changes
// the transcribed text. The URL hash selects the tab, so links can open one.
export const Corrections = () => {
    const { t } = useTranslation();
    const { hash } = useLocation();
    const navigate = useNavigate();
    const activeTab = tabFromHash(hash);
    const [showBuiltIn, setShowBuiltIn] = useShowBuiltInRules();
    const [anchor, setAnchor] = useState<string | null>(null);

    const goToTab = (tab: CorrectionsTab) => void navigate({ to: CORRECTIONS_PATH, hash: tab, replace: true });

    const selectTab = (tab: CorrectionsTab) => {
        setAnchor(null);
        goToTab(tab);
    };

    const open = (target: CorrectionsTarget) => {
        if (target.builtIn === true) {
            setShowBuiltIn(true);
        }
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
                    { value: 'dictionary', label: t('Personal dictionary') },
                    { value: 'rules', label: t('Rules') },
                ]}
            >
                {activeTab === 'explanation' && <CorrectionsExplanation onOpen={open} />}
                {activeTab === 'dictionary' && <CustomDictionary />}
                {activeTab === 'rules' && (
                    <FormattingRules showBuiltIn={showBuiltIn} onShowBuiltInChange={setShowBuiltIn} anchor={anchor} />
                )}
            </Tabs>
        </main>
    );
};
