import { useEffect, useState } from 'react';
import { Typography } from '@/components/typography';
import { Switch } from '@/components/switch';
import { Slider } from '@/components/slider';
import { useTranslation } from '@/i18n';
import { useFormattingRules } from './hooks/use-formatting-rules';
import { useRemoveHesitations } from './hooks/use-remove-hesitations';
import { RuleCard } from '../../../components/rule-card';
import { AddRuleSection } from '../../../components/add-rule-section';
import { Select, SelectTrigger, SelectValue, SelectContent, SelectItem } from '../../../components/select';
import { NumberInput } from '@/components/number-input';
import { SettingsUI } from '@/components/settings-ui';
import {
    DndContext,
    closestCenter,
    DragEndEvent,
    DragStartEvent,
    DragOverlay,
    PointerSensor,
    KeyboardSensor,
    useSensor,
    useSensors,
} from '@dnd-kit/core';
import {
    SortableContext,
    verticalListSortingStrategy,
    useSortable,
    sortableKeyboardCoordinates,
} from '@dnd-kit/sortable';
import { CSS } from '@dnd-kit/utilities';
import { restrictToVerticalAxis } from '@dnd-kit/modifiers';
import { FormattingRule } from './types';
import { MedicalRules } from '../medical-rules/medical-rules';
import { MedicalDataSettings } from '@/features/medical-data/medical-data-settings/medical-data-settings';
import { RULES_ANCHORS } from '@/features/corrections/corrections.helpers';

interface SortableRuleCardProps {
    rule: FormattingRule;
    onUpdate: (id: string, updates: Partial<Omit<FormattingRule, 'id'>>) => void;
    onDelete: (id: string) => void;
    onDuplicate: (id: string) => void;
}

const SortableRuleCard = ({ rule, ...props }: SortableRuleCardProps) => {
    const { attributes, listeners, setNodeRef, transform, transition, isDragging } = useSortable({ id: rule.id });

    const style = {
        transform: CSS.Transform.toString(transform),
        transition,
    };

    return (
        <div ref={setNodeRef} style={style}>
            {isDragging ? (
                <div className="border border-dashed border-border rounded-lg h-14 bg-card/10" />
            ) : (
                <RuleCard rule={rule} {...props} dragHandleProps={{ ...attributes, ...listeners }} />
            )}
        </div>
    );
};

interface FormattingRulesProps {
    /** Also list the built-in options and the bundled medical rules. */
    showBuiltIn: boolean;
    onShowBuiltInChange: (show: boolean) => void;
    /** Id of the block to scroll to once loaded (see `RULES_ANCHORS`). */
    anchor: string | null;
}

/** Rules tab of the Corrections page: the user's rules, plus the built-in ones on request. */
export const FormattingRules = ({ showBuiltIn, onShowBuiltInChange, anchor }: FormattingRulesProps) => {
    const { t } = useTranslation();
    const { settings, isLoading, updateBuiltInOption, addRule, updateRule, deleteRule, duplicateRule, reorderRules } =
        useFormattingRules();
    const { removeHesitations, setRemoveHesitations } = useRemoveHesitations();

    const shortTextThreshold = settings.built_in.short_text_correction;
    const [activeId, setActiveId] = useState<string | null>(null);

    const sensors = useSensors(
        useSensor(PointerSensor, {
            activationConstraint: { distance: 8 },
        }),
        useSensor(KeyboardSensor, {
            coordinateGetter: sortableKeyboardCoordinates,
        })
    );

    const handleDragStart = (event: DragStartEvent) => {
        setActiveId(String(event.active.id));
    };

    const handleDragEnd = (event: DragEndEvent) => {
        setActiveId(null);
        const { active, over } = event;
        if (over != null && active.id !== over.id) {
            reorderRules(String(active.id), String(over.id));
        }
    };

    const activeRule = (activeId != null && settings.rules.find((rule) => rule.id === activeId)) || undefined;

    useEffect(() => {
        if (anchor == null || isLoading) {
            return;
        }
        document.getElementById(anchor)?.scrollIntoView({ behavior: 'smooth', block: 'center' });
    }, [anchor, isLoading, showBuiltIn]);

    if (isLoading) {
        return <div className="text-muted-foreground">{t('Loading...')}</div>;
    }

    return (
        <div className="space-y-6">
            <SettingsUI.Container>
                <SettingsUI.Item>
                    <SettingsUI.Description className="w-150">
                        <Typography.Title>{t('Show the built-in rules')}</Typography.Title>
                        <Typography.Paragraph>
                            {t(
                                'Also lists the built-in options, the medical data updates and a search of the bundled drug and medical term rules. Your own rules are always shown.'
                            )}
                        </Typography.Paragraph>
                    </SettingsUI.Description>
                    <Switch
                        checked={showBuiltIn}
                        onCheckedChange={onShowBuiltInChange}
                        data-testid="show-built-in-rules"
                    />
                </SettingsUI.Item>
            </SettingsUI.Container>

            <SettingsUI.Section
                id={RULES_ANCHORS.userRules}
                title={t('Your formatting rules')}
                titleTestId="formatting-rules-title"
                aside={<span className="text-xs text-muted-foreground">{settings.rules.length}</span>}
            >
                <div className="space-y-4 p-4">
                    <Typography.Paragraph>
                        {t('Replace words or patterns with your own text. Rules apply from top to bottom.')}
                        {settings.rules.length === 0 && (
                            <>
                                <br />
                                <span className="text-xs italic text-muted-foreground">
                                    {t('Example: "semicolon" → ";"')}
                                </span>
                            </>
                        )}
                    </Typography.Paragraph>
                    {settings.rules.length > 0 && (
                        <DndContext
                            sensors={sensors}
                            collisionDetection={closestCenter}
                            onDragStart={handleDragStart}
                            onDragEnd={handleDragEnd}
                            modifiers={[restrictToVerticalAxis]}
                        >
                            <SortableContext
                                items={settings.rules.map((rule) => rule.id)}
                                strategy={verticalListSortingStrategy}
                            >
                                <div className="space-y-3">
                                    {settings.rules.map((rule) => (
                                        <SortableRuleCard
                                            key={rule.id}
                                            rule={rule}
                                            onUpdate={updateRule}
                                            onDelete={deleteRule}
                                            onDuplicate={duplicateRule}
                                        />
                                    ))}
                                </div>
                            </SortableContext>
                            <DragOverlay>
                                {activeRule != null && (
                                    <RuleCard
                                        rule={activeRule}
                                        onUpdate={updateRule}
                                        onDelete={deleteRule}
                                        onDuplicate={duplicateRule}
                                    />
                                )}
                            </DragOverlay>
                        </DndContext>
                    )}
                    <AddRuleSection onAdd={addRule} />
                </div>
            </SettingsUI.Section>

            {settings.built_in.medical_fixes && (
                <div id={RULES_ANCHORS.medicalRules}>
                    <MedicalRules showBuiltIn={showBuiltIn} />
                </div>
            )}

            {showBuiltIn && (
                <section id={RULES_ANCHORS.builtInOptions} className="space-y-3" data-testid="built-in-options">
                    <Typography.Title>{t('Built-in Options')}</Typography.Title>
                    <SettingsUI.Container id={RULES_ANCHORS.medicalFixes}>
                        <SettingsUI.Item>
                            <SettingsUI.Description className="w-150">
                                <Typography.Title>{t('Fix drug names and medical terms')}</Typography.Title>
                                <Typography.Paragraph>
                                    {t(
                                        'Corrects misheard French drug names and medical terms with about 25,000 bundled rules.'
                                    )}
                                    <span className="block text-xs text-muted-foreground mt-1">
                                        {t('Example: "tacadol" → "Takadol"')}
                                    </span>
                                </Typography.Paragraph>
                            </SettingsUI.Description>
                            <Switch
                                checked={settings.built_in.medical_fixes}
                                onCheckedChange={(checked) => updateBuiltInOption('medical_fixes', checked)}
                                data-testid="option-medical-fixes"
                            />
                        </SettingsUI.Item>
                    </SettingsUI.Container>
                    <div id={RULES_ANCHORS.medicalData}>
                        <MedicalDataSettings />
                    </div>

                    <SettingsUI.Container id={RULES_ANCHORS.phraseBoosting}>
                        <SettingsUI.Item>
                            <SettingsUI.Description className="w-150">
                                <Typography.Title>{t('Boost medical vocabulary')}</Typography.Title>
                                <Typography.Paragraph>
                                    {t(
                                        'Helps the model recognize about 77,000 French medical words and phrases while you speak.'
                                    )}
                                </Typography.Paragraph>
                            </SettingsUI.Description>
                            <Switch
                                checked={settings.built_in.phrase_boosting}
                                onCheckedChange={(checked) => updateBuiltInOption('phrase_boosting', checked)}
                                data-testid="option-phrase-boosting"
                            />
                        </SettingsUI.Item>
                    </SettingsUI.Container>

                    <SettingsUI.Container id={RULES_ANCHORS.confusionRescoring}>
                        <SettingsUI.Item>
                            <SettingsUI.Description className="w-150">
                                <Typography.Title>{t('Resolve sound-alike medical words')}</Typography.Title>
                                <Typography.Paragraph>
                                    {t(
                                        'When a word sounds like several medical terms, listens to that passage again and keeps the one the audio supports.'
                                    )}
                                    <br />
                                    <span className="text-xs italic text-muted-foreground">
                                        {t('Example: "singulaire" → "Singulair" or "cingulaire"')}
                                    </span>
                                </Typography.Paragraph>
                            </SettingsUI.Description>
                            <Switch
                                checked={settings.built_in.confusion_rescoring}
                                onCheckedChange={(checked) => updateBuiltInOption('confusion_rescoring', checked)}
                                data-testid="option-confusion-rescoring"
                            />
                        </SettingsUI.Item>
                    </SettingsUI.Container>

                    <SettingsUI.Container id={RULES_ANCHORS.shortText}>
                        <SettingsUI.Item>
                            <SettingsUI.Description className="w-150">
                                <Typography.Title>{t('Short text correction')}</Typography.Title>
                                <Typography.Paragraph>
                                    {shortTextThreshold > 0 ? (
                                        <>
                                            {t(
                                                'Removes capitalization and punctuation for transcriptions of {{count}} word(s) or fewer.',
                                                { count: shortTextThreshold }
                                            )}
                                            <br />
                                            <span className="text-xs italic text-muted-foreground">
                                                {t('Example: "Hello." → "hello"')}
                                            </span>
                                        </>
                                    ) : (
                                        <span className="italic text-muted-foreground">
                                            {t('Disabled. Move the slider to activate.')}
                                        </span>
                                    )}
                                </Typography.Paragraph>
                            </SettingsUI.Description>
                            <Slider
                                value={[shortTextThreshold]}
                                onValueChange={([value]) => updateBuiltInOption('short_text_correction', value)}
                                min={0}
                                max={5}
                                step={1}
                                showValue
                                className="w-28"
                                data-testid="option-short-text-correction"
                            />
                        </SettingsUI.Item>
                    </SettingsUI.Container>

                    <SettingsUI.Container id={RULES_ANCHORS.formattingOptions}>
                        <SettingsUI.Item>
                            <SettingsUI.Description className="w-150">
                                <Typography.Title>{t('Add space before ? and !')}</Typography.Title>
                                <Typography.Paragraph>
                                    {t(
                                        'Automatically adds a space before question marks and exclamation points if missing.'
                                    )}
                                    <br />
                                    <span className="text-xs italic text-muted-foreground">
                                        {t('Example: "Hello?" → "Hello ?"')}
                                    </span>
                                </Typography.Paragraph>
                            </SettingsUI.Description>
                            <Switch
                                checked={settings.built_in.space_before_punctuation}
                                onCheckedChange={(checked) => updateBuiltInOption('space_before_punctuation', checked)}
                                data-testid="option-space-before-punctuation"
                            />
                        </SettingsUI.Item>
                    </SettingsUI.Container>

                    <SettingsUI.Container>
                        <SettingsUI.Item>
                            <SettingsUI.Description className="w-150">
                                <Typography.Title>{t('Add space at end of transcription')}</Typography.Title>
                                <Typography.Paragraph>
                                    {t(
                                        'Ensures each transcription ends with a space. Prevents consecutive transcriptions from "sticking" together.'
                                    )}
                                </Typography.Paragraph>
                            </SettingsUI.Description>
                            <Switch
                                checked={settings.built_in.trailing_space}
                                onCheckedChange={(checked) => updateBuiltInOption('trailing_space', checked)}
                                data-testid="option-trailing-space"
                            />
                        </SettingsUI.Item>
                    </SettingsUI.Container>

                    <SettingsUI.Container>
                        <SettingsUI.Item>
                            <SettingsUI.Description className="w-150">
                                <Typography.Title>{t('Convert text numbers to digits')}</Typography.Title>
                                <Typography.Paragraph>
                                    {t('Automatically converts numbers written in letters to digits.')}
                                    <br />
                                    <span className="text-xs italic text-muted-foreground">
                                        {t('Example: "one" → "1", "twenty-three" → "23"')}
                                    </span>
                                </Typography.Paragraph>
                            </SettingsUI.Description>
                            <Switch
                                checked={settings.built_in.convert_text_numbers}
                                onCheckedChange={(checked) => updateBuiltInOption('convert_text_numbers', checked)}
                                data-testid="option-convert-text-numbers"
                            />
                        </SettingsUI.Item>
                        {settings.built_in.convert_text_numbers && (
                            <>
                                <SettingsUI.Separator />
                                <SettingsUI.Item>
                                    <SettingsUI.Description className="flex-1">
                                        <Typography.Title>{t('Language for number conversion')}</Typography.Title>
                                        <Typography.Paragraph>
                                            {t('Choose the language for text-to-number conversion')}
                                        </Typography.Paragraph>
                                    </SettingsUI.Description>
                                    <Select
                                        value={settings.built_in.text_numbers_language}
                                        onValueChange={(value) => updateBuiltInOption('text_numbers_language', value)}
                                    >
                                        <SelectTrigger className="w-45">
                                            <SelectValue placeholder={t('Select language')} />
                                        </SelectTrigger>
                                        <SelectContent>
                                            <SelectItem value="en">English</SelectItem>
                                            <SelectItem value="fr">Français</SelectItem>
                                            <SelectItem value="de">Deutsch</SelectItem>
                                            <SelectItem value="it">Italiano</SelectItem>
                                            <SelectItem value="es">Español</SelectItem>
                                            <SelectItem value="nl">Nederlands</SelectItem>
                                            <SelectItem value="pt">Português</SelectItem>
                                        </SelectContent>
                                    </Select>
                                </SettingsUI.Item>
                                <SettingsUI.Separator />
                                <SettingsUI.Item>
                                    <SettingsUI.Description className="w-150">
                                        <Typography.Title>{t('Conversion threshold')}</Typography.Title>
                                        <Typography.Paragraph>
                                            {t('Do not convert numbers that are strictly below this threshold.')}
                                        </Typography.Paragraph>
                                    </SettingsUI.Description>
                                    <div className="w-30">
                                        <NumberInput
                                            value={settings.built_in.text_numbers_threshold}
                                            onValueChange={(value) =>
                                                updateBuiltInOption('text_numbers_threshold', value ?? 10)
                                            }
                                            min={0}
                                            max={50}
                                        />
                                    </div>
                                </SettingsUI.Item>
                            </>
                        )}
                    </SettingsUI.Container>

                    <SettingsUI.Container id={RULES_ANCHORS.hesitations}>
                        <SettingsUI.Item>
                            <SettingsUI.Description className="w-150">
                                <Typography.Title>{t('Remove hesitation sounds')}</Typography.Title>
                                <Typography.Paragraph>
                                    {t('Removes sounds like "euh", "um" and "hmm" from transcriptions.')}
                                    <br />
                                    <span className="text-xs italic text-muted-foreground">
                                        {t('Example: "I um think so" → "I think so"')}
                                    </span>
                                </Typography.Paragraph>
                            </SettingsUI.Description>
                            <Switch
                                checked={removeHesitations}
                                onCheckedChange={setRemoveHesitations}
                                data-testid="option-remove-hesitations"
                            />
                        </SettingsUI.Item>
                    </SettingsUI.Container>
                </section>
            )}
        </div>
    );
};
