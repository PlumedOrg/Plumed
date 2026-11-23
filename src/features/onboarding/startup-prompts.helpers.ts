// Plumed fork (written with Claude Code): the startup questions show one per
// launch, in this order: the speech model download (always first, see
// src/features/model), PhoneMic, usage statistics, medical data check, then
// the medical data update offer. Each later one waits for a launch where no
// earlier one is due.
import { invoke } from '@tauri-apps/api/core';
import { AppSettings } from '@/features/settings/settings.types';
import { isModelMissing } from '@/features/model/model.helpers';
import { ModelStatus } from '@/features/model/types';

/** Whether the model download dialog opens this launch: it beats every other question. */
export const isModelPromptDue = async (): Promise<boolean> =>
    isModelMissing(await invoke<ModelStatus>('get_model_status'));

/** Whether the model, PhoneMic or usage statistics question is asked this launch. */
export const isEarlierStartupPromptDue = async (): Promise<boolean> => {
    const [settings, modelDue, remoteMicDue, telemetryAvailable] = await Promise.all([
        invoke<AppSettings>('get_all_settings'),
        isModelPromptDue(),
        invoke<boolean>('remote_mic_prompt_due'),
        invoke<boolean>('is_telemetry_available'),
    ]);
    return modelDue || remoteMicDue || (telemetryAvailable && !settings.telemetry_asked);
};
