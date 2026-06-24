import {
  shouldRecordSuccessfulCaptureSelection,
  type CaptureCompletionAction,
  type CaptureSelectionFlow,
} from './captureActions';
import {
  getCaptureModeSelectionFlow as getRuntimeCaptureModeSelectionFlow,
  getPrimaryCaptureCompletionActionForMode as getRuntimePrimaryCaptureCompletionActionForMode,
  planCandidateSelectionCompletion,
} from './captureInteractionRuntime';
import type { CaptureMode } from './types';

export type CaptureCompletionEffect =
  | 'copy'
  | 'save'
  | 'quick-save'
  | 'pin'
  | 'ocr'
  | 'print'
  | 'cancel';

export type CaptureResultWindow = 'ocr' | 'translation';

export interface CaptureCompletionPlan {
  action: CaptureCompletionAction;
  effect: CaptureCompletionEffect;
  resultWindow: CaptureResultWindow | null;
  shouldRecordSelection: boolean;
  shouldFinishSession: boolean;
}

export function shouldRecordSuccessfulCaptureCompletion(
  action: CaptureCompletionAction,
) {
  return shouldRecordSuccessfulCaptureSelection(action);
}

export function getCaptureModeSelectionFlow(
  mode: CaptureMode,
): CaptureSelectionFlow {
  return getRuntimeCaptureModeSelectionFlow(mode);
}

export function getPrimaryCaptureCompletionActionForMode(
  mode: CaptureMode,
): CaptureCompletionAction {
  return getRuntimePrimaryCaptureCompletionActionForMode(mode);
}

export function getCaptureCompletionPlan(
  action: CaptureCompletionAction,
): CaptureCompletionPlan {
  const effects = planCandidateSelectionCompletion(action);
  const outputEffect = effects.find((effect) => effect.type === 'output-capture');
  const ocrEffect = effects.find((effect) => effect.type === 'run-ocr');

  return {
    action,
    effect: outputEffect?.action ?? (ocrEffect ? 'ocr' : 'cancel'),
    resultWindow: ocrEffect?.resultWindow ?? null,
    shouldRecordSelection: effects.some((effect) => effect.type === 'record-selection'),
    shouldFinishSession: effects.some((effect) => effect.type === 'finish-session'),
  };
}
