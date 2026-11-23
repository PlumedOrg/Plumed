import { ReactNode, useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { GraduationCap } from 'lucide-react';
import { toast } from 'react-toastify';
import { Page } from '@/components/page';
import { Typography } from '@/components/typography';
import { useTranslation } from '@/i18n';
import { AppSettings } from '@/features/settings/settings.types';
import {
    CONTEXT_EXAMPLE,
    LAYOUT_COMMANDS,
    PUNCTUATION_COMMANDS,
    SHORTHAND_COMMANDS,
    SpokenCommand,
    TEMPLATE_NAMES,
} from './tutorial.data';

interface SectionProps {
    title: string;
    intro?: string;
    children: ReactNode;
}

const Section = ({ title, intro, children }: SectionProps) => (
    <section className="space-y-2 rounded-md border border-border bg-background p-4">
        <Typography.Title>{title}</Typography.Title>
        {intro !== undefined && <Typography.Paragraph>{intro}</Typography.Paragraph>}
        {children}
    </section>
);

const CommandTable = ({ commands }: { commands: SpokenCommand[] }) => (
    <ul className="grid grid-cols-1 gap-x-6 gap-y-1 text-sm sm:grid-cols-2">
        {commands.map((command) => (
            <li key={command.say} className="flex items-baseline justify-between gap-3">
                <span>« {command.say} »</span>
                <code className="rounded bg-muted px-1.5 py-0.5 text-xs">{command.writes}</code>
            </li>
        ))}
    </ul>
);

// Plumed fork (written with Claude Code): a short guide to dictating well,
// shown on first launch. The spoken commands are the ones the default formatting
// rules support (see tutorial.data.ts). Finishing it is stored locally as
// `tutorial_completed`, and sent as a yes/no in the opt-in usage statistics.
export const Tutorial = () => {
    const { t } = useTranslation();
    const [completed, setCompleted] = useState<boolean | null>(null);

    useEffect(() => {
        invoke('mark_tutorial_opened').catch((e) => console.error('Failed to mark the tutorial as opened:', e));
        invoke<AppSettings>('get_all_settings')
            .then((settings) => setCompleted(settings.tutorial_completed))
            .catch((e) => console.error('Failed to load the tutorial state:', e));
    }, []);

    const markCompleted = async () => {
        try {
            await invoke('set_tutorial_completed', { completed: true });
            setCompleted(true);
        } catch (e) {
            toast.error(t('Failed to save the tutorial state'));
            console.error(e);
        }
    };

    return (
        <main className="space-y-4" data-testid="tutorial-page">
            <Page.Header>
                <Typography.MainTitle className="flex items-center gap-3">
                    <GraduationCap className="w-6 h-6 text-sky-400" />
                    {t('Tutorial')}
                </Typography.MainTitle>
                <Typography.Paragraph>
                    {t('Five minutes to dictate well. You can come back to this page at any time.')}
                </Typography.Paragraph>
            </Page.Header>

            <Section
                title={t('Dictate whole sentences')}
                intro={t(
                    'The speech model needs context to pick the right word. A single word is ambiguous: "venlafaxine" alone can come out as "Vend la faxine". In a sentence, it is understood.'
                )}
            >
                <ul className="space-y-1 text-sm">
                    <li>
                        <span className="text-muted-foreground">{t('Too short')}</span> « {CONTEXT_EXAMPLE.tooShort} »
                    </li>
                    <li>
                        <span className="text-emerald-400">{t('Better')}</span> « {CONTEXT_EXAMPLE.full} »
                    </li>
                </ul>
            </Section>

            <Section
                title={t('Line breaks')}
                intro={t('Say the command out loud, in the middle of your dictation. Plumed replaces it.')}
            >
                <CommandTable commands={LAYOUT_COMMANDS} />
            </Section>

            <Section title={t('Punctuation')}>
                <CommandTable commands={PUNCTUATION_COMMANDS} />
            </Section>

            <Section
                title={t('Medical shorthand')}
                intro={t('A few of the provided rules. You can see and edit all of them in Corrections.')}
            >
                <CommandTable commands={SHORTHAND_COMMANDS} />
            </Section>

            <Section
                title={t('Ready-made notes')}
                intro={t(
                    'Say "Ajoute la commande" followed by a name to insert a whole note, for example one of these:'
                )}
            >
                <ul className="flex flex-wrap gap-2 text-sm">
                    {TEMPLATE_NAMES.map((name) => (
                        <li key={name} className="rounded bg-muted px-2 py-1">
                            {name}
                        </li>
                    ))}
                </ul>
            </Section>

            {/* Hidden once finished (and while loading): there is nothing left to do with it. */}
            {completed === false && (
                <div className="flex justify-end">
                    <Page.PrimaryButton onClick={() => void markCompleted()} data-testid="tutorial-complete">
                        {t("I've finished the tutorial")}
                    </Page.PrimaryButton>
                </div>
            )}
        </main>
    );
};
