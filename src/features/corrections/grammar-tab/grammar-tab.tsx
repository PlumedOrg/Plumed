import { useState } from 'react';
import clsx from 'clsx';
import { Input } from '@/components/input';
import { SettingsUI } from '@/components/settings-ui';
import { Switch } from '@/components/switch';
import { Typography } from '@/components/typography';
import { useTranslation } from '@/i18n';
import { useFormattingRules } from '@/features/personalize/formatting-rules/hooks/use-formatting-rules';
import { pipelineStepTitle } from '../corrections.helpers';
import { useListFile } from '../hooks/use-list-file';
import { ListHeader } from '../list-header/list-header';
import { PAGE_SIZE, Pagination } from '../pagination/pagination';
import { changeCount, enabledIdsWith, filterRules, isRisky, measureLabel } from './grammar-tab.helpers';
import { GrammarRule } from './grammar-tab.types';
import { useGrammarRules } from './hooks/use-grammar-rules';

interface RuleRowProps {
    rule: GrammarRule;
    onToggle: (rule: GrammarRule, enabled: boolean) => void;
}

const RuleRow = ({ rule, onToggle }: RuleRowProps) => {
    const { t } = useTranslation();
    return (
        <div className="flex items-center gap-2 py-1.5" data-testid={`grammar-rule-${rule.id}`}>
            <div className="flex-1 space-y-0.5">
                <div className="font-mono text-xs">{rule.id}</div>
                <div className={clsx('text-xs', isRisky(rule) ? 'text-amber-400' : 'text-muted-foreground')}>
                    {measureLabel(rule, t)}
                </div>
            </div>
            <Switch
                checked={rule.enabled}
                onCheckedChange={(enabled) => onToggle(rule, enabled)}
                aria-label={t('Enabled')}
            />
        </div>
    );
};

/**
 * "Grammar" tab (Plumed fork, written with Claude Code): Grammalecte's fixes with a
 * single suggestion, for the rules chosen here. The 8 rules measured as helpful are on
 * by default; the other grammar families are listed off, with what they did on the
 * evaluation data, so turning one on is an informed choice.
 */
export const GrammarTab = () => {
    const { t } = useTranslation();
    const { settings, updateBuiltInOption } = useFormattingRules();
    const { data, reload, saveEnabled, reset } = useGrammarRules();
    const { exportList, importList } = useListFile('grammar', reload);
    const [query, setQuery] = useState('');
    const [offset, setOffset] = useState(0);

    const toggle = (rule: GrammarRule, enabled: boolean) => saveEnabled(enabledIdsWith(data.rules, rule.id, enabled));
    const defaults = data.rules.filter((rule) => rule.default);
    const others = filterRules(
        data.rules.filter((rule) => !rule.default),
        query
    );
    const search = (text: string) => {
        setQuery(text);
        setOffset(0);
    };

    return (
        <div className="space-y-6">
            <ListHeader
                title={pipelineStepTitle('grammar', t)}
                description={t(
                    'Last step: the Grammalecte grammar checker reads the final text and Plumed applies its fixes when there is only one possible fix and the rule is on below. Ages, doses, acronyms and names after "Madame" or "Docteur" are hidden from it, so it does not take them for agreement errors. Typography (spaces, apostrophes, capitals) is never changed.'
                )}
                option={{
                    checked: settings.built_in.grammar,
                    onChange: (checked) => updateBuiltInOption('grammar', checked),
                    testId: 'option-grammar',
                }}
                summary={changeCount(data.rules)}
                resetDescription={t(
                    'The recommended rules are turned back on and every other rule is turned off. This cannot be undone.'
                )}
                onReset={reset}
                onExport={exportList}
                onImport={importList}
                testId="grammar"
            />

            <SettingsUI.Section title={t('Recommended rules')} titleTestId="grammar-defaults-title">
                <div className="space-y-2 p-4">
                    <Typography.Paragraph>
                        {t(
                            'On by default: on medical dictations, their fixes made the text better far more often than worse.'
                        )}
                    </Typography.Paragraph>
                    <div className="divide-y divide-surface-border" data-testid="grammar-defaults">
                        {defaults.map((rule) => (
                            <RuleRow key={rule.id} rule={rule} onToggle={toggle} />
                        ))}
                    </div>
                </div>
            </SettingsUI.Section>

            <SettingsUI.Section
                title={t('Other grammar rules')}
                titleTestId="grammar-others-title"
                aside={<span className="text-xs text-muted-foreground">{others.length.toLocaleString()}</span>}
            >
                <div className="space-y-4 p-4">
                    <Typography.Paragraph>
                        {t(
                            'Off by default. The figures say how often each family of rules made a transcription better or worse on the evaluation dictations: below 80 % right (in orange), it does more harm than good on medical text.'
                        )}
                    </Typography.Paragraph>
                    <Input
                        value={query}
                        onChange={(e) => search(e.target.value)}
                        placeholder={t('Search a rule, e.g. ppas')}
                        data-testid="grammar-search"
                    />
                    <div className="divide-y divide-surface-border" data-testid="grammar-others">
                        {others.slice(offset, offset + PAGE_SIZE).map((rule) => (
                            <RuleRow key={rule.id} rule={rule} onToggle={toggle} />
                        ))}
                        {others.length === 0 && <p className="py-2 text-sm text-muted-foreground">{t('No results')}</p>}
                    </div>
                    <Pagination offset={offset} total={others.length} onOffsetChange={setOffset} />
                </div>
            </SettingsUI.Section>

            <p className="text-xs text-muted-foreground" data-testid="grammar-credit">
                {t(
                    'Grammalecte {{version}} by Olivier R. (GNU GPL v3), embedded in Plumed: the check runs on your computer, nothing is sent.',
                    { version: data.grammalecte_version }
                )}
            </p>
        </div>
    );
};
