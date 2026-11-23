import { useState } from 'react';
import { Input } from '@/components/input';
import { Typography } from '@/components/typography';
import { useTranslation } from '@/i18n';
import { useFormattingRules } from './hooks/use-formatting-rules';
import { RuleCard } from '../../../components/rule-card';
import { AddRuleSection } from '../../../components/add-rule-section';
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
import { filterFormattingRules } from './formatting-rules.helpers';
import { useUserRulesCollapsed } from '@/features/corrections/hooks/use-user-rules-collapsed';

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
    /** The settings, shared with the tab's header (reset, import). */
    formatting: ReturnType<typeof useFormattingRules>;
}

/** Formatting tab of the Corrections page: the user's rules, in a list that can be folded. */
export const FormattingRules = ({ formatting }: FormattingRulesProps) => {
    const { t } = useTranslation();
    const { settings, isLoading, addRule, updateRule, deleteRule, duplicateRule, reorderRules } = formatting;
    const [collapsed, setCollapsed] = useUserRulesCollapsed();
    const [activeId, setActiveId] = useState<string | null>(null);
    const [query, setQuery] = useState('');

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

    const visibleRules = filterFormattingRules(settings.rules, query);

    if (isLoading) {
        return <div className="text-muted-foreground">{t('Loading...')}</div>;
    }

    return (
        <div className="space-y-6">
            <SettingsUI.Section
                title={t('Your formatting rules')}
                collapsed={collapsed}
                onToggle={() => setCollapsed(!collapsed)}
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
                        <Input
                            value={query}
                            onChange={(e) => setQuery(e.target.value)}
                            placeholder={t('Search a rule, e.g. mg')}
                            data-testid="formatting-rules-search"
                        />
                    )}
                    {settings.rules.length > 0 && visibleRules.length === 0 && (
                        <p className="text-sm text-muted-foreground">{t('No results')}</p>
                    )}
                    {visibleRules.length > 0 && (
                        <DndContext
                            sensors={sensors}
                            collisionDetection={closestCenter}
                            onDragStart={handleDragStart}
                            onDragEnd={handleDragEnd}
                            modifiers={[restrictToVerticalAxis]}
                        >
                            <SortableContext
                                items={visibleRules.map((rule) => rule.id)}
                                strategy={verticalListSortingStrategy}
                            >
                                <div className="space-y-3">
                                    {visibleRules.map((rule) => (
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
        </div>
    );
};
