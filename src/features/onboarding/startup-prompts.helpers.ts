// Plumed fork (written with Claude Code): the startup wizard asks every first
// launch question in one dialog (see startup-wizard/). A step is only due while
// its question is unanswered, so users who already answered are not asked again.
import { invoke } from '@tauri-apps/api/core';
import { AppSettings } from '@/features/settings/settings.types';
import { isModelMissing } from '@/features/model/model.helpers';
import { ModelStatus } from '@/features/model/types';

export type StartupStep = 'model' | 'remote-mic' | 'telemetry' | 'medical-data' | 'update-channel';

interface TelemetryQuestion {
    due: boolean;
    offer_never_again: boolean;
}

export interface DueSteps {
    steps: StartupStep[];
    /** The third usage statistics ask also offers "Never ask again". */
    offerTelemetryNeverAgain: boolean;
}

/** The unanswered questions, in the order the wizard asks them. */
export const getDueSteps = async (): Promise<DueSteps> => {
    const [settings, modelStatus, remoteMicDue, telemetry, medicalDataDue] = await Promise.all([
        invoke<AppSettings>('get_all_settings'),
        invoke<ModelStatus>('get_model_status'),
        invoke<boolean>('remote_mic_prompt_due'),
        invoke<TelemetryQuestion>('get_telemetry_question'),
        invoke<boolean>('medical_data_question_due'),
    ]);
    const due: [StartupStep, boolean][] = [
        ['model', isModelMissing(modelStatus)],
        ['remote-mic', remoteMicDue],
        ['telemetry', telemetry.due],
        ['medical-data', medicalDataDue],
        ['update-channel', !settings.app_update_channel_asked],
    ];
    return {
        steps: due.filter(([, isDue]) => isDue).map(([step]) => step),
        offerTelemetryNeverAgain: telemetry.offer_never_again,
    };
};

/** Whether the wizard opens this launch (the medical data offer then waits for the next one). */
export const isStartupWizardDue = async (): Promise<boolean> => (await getDueSteps()).steps.length > 0;
