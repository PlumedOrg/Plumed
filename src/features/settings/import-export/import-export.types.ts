import type { ComponentType, ReactNode } from 'react';
import { BuiltInOptions, FormattingRule } from '@/features/personalize/formatting-rules/types';

export type { SystemSettings, ShortcutSettings, VoiceModeSettings, AppSettings } from '../settings.types';
import type { SystemSettings, ShortcutSettings, VoiceModeSettings } from '../settings.types';

export type ExportedSystemSettings = Omit<
    SystemSettings,
    | 'wayland_notice_dismissed'
    | 'wayland_clipboard_fallback_dismissed'
    | 'keep_recordings'
    | 'remove_hesitations'
    | 'block_cli_import'
    | 'short_dictation_context'
    | 'remote_mic_enabled'
    | 'remote_mic_url'
    | 'remote_mic_prompt_version'
    | 'telemetry_enabled'
    | 'telemetry_asked'
    | 'medical_data_update_check'
    | 'medical_data_skipped_version'
    | 'medical_data_update_asked'
>;

export interface MurmureExportData {
    version: number;
    app_version: string;
    exported_at: string;
    categories: ExportedCategories;
}

export interface ExportedCategories {
    settings?: ExportedSystemSettings;
    shortcuts?: ShortcutSettings;
    voice_mode?: VoiceModeSettings;
    formatting_rules?: ExportedFormattingSettings;
    dictionary?: string[];
}

export interface ExportedFormattingSettings {
    built_in?: BuiltInOptions;
    rules: FormattingRule[];
}

export type CategoryKey = 'settings' | 'shortcuts' | 'voice_mode' | 'formatting_rules' | 'dictionary';

export type ImportStrategy = 'replace' | 'merge';

export type ImportState =
    | 'idle'
    | 'previewing'
    | 'importing'
    | 'done'
    | 'partial_error'
    | 'file_error'
    | 'version_error';

export interface CategorySubItem {
    key: string;
    label: string;
}

export type DynamicSubItemsRenderer = (props: {
    selection: Record<string, boolean>;
    onToggle: (key: string, checked: boolean) => void;
    disabled?: boolean;
}) => ReactNode;

export interface CategoryDefinition {
    key: CategoryKey;
    label: string;
    icon: ComponentType<{ className?: string }>;
    subItems: CategorySubItem[];
    supportsMerge: boolean;
    dynamicSubItems?: DynamicSubItemsRenderer;
}

export interface CategorySelection {
    [categoryKey: string]: {
        selected: boolean;
        subItems: Record<string, boolean>;
    };
}
