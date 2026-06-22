import {
  getCaptureSelectionFlowForMode,
  shouldRecordSuccessfulCaptureSelection,
  type CaptureCompletionAction,
  type CaptureSelectionFlow,
} from './captureActions';
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
  return getCaptureSelectionFlowForMode(mode);
}

export function getPrimaryCaptureCompletionActionForMode(
  mode: CaptureMode,
): CaptureCompletionAction {
  const flow = getCaptureModeSelectionFlow(mode);
  if (flow === 'ocr' || flow === 'ocr-translate') return flow;
  return 'copy';
}

export function getCaptureCompletionPlan(
  action: CaptureCompletionAction,
): CaptureCompletionPlan {
  return {
    action,
    effect: getCaptureCompletionEffect(action),
    resultWindow: getCaptureCompletionResultWindow(action),
    shouldRecordSelection: shouldRecordSuccessfulCaptureCompletion(action),
    shouldFinishSession: action !== 'cancel',
  };
}

function getCaptureCompletionEffect(
  action: CaptureCompletionAction,
): CaptureCompletionEffect {
  if (action === 'ocr-translate') return 'ocr';
  return action;
}

function getCaptureCompletionResultWindow(
  action: CaptureCompletionAction,
): CaptureResultWindow | null {
  if (action === 'ocr') return 'ocr';
  if (action === 'ocr-translate') return 'translation';
  return null;
}
