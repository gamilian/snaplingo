import {
  getCaptureSelectionFlowForMode,
  shouldRecordSuccessfulCaptureSelection,
  type CaptureCompletionAction,
  type CaptureSelectionFlow,
} from './captureActions';
import type { CaptureMode } from './types';

export type CaptureRuntimeOutputAction =
  | 'copy'
  | 'save'
  | 'quick-save'
  | 'pin'
  | 'print';

export type CaptureRuntimeResultWindow = 'ocr' | 'translation' | null;

export type CaptureRuntimeEffect =
  | { type: 'output-capture'; action: CaptureRuntimeOutputAction }
  | { type: 'run-ocr'; resultWindow: CaptureRuntimeResultWindow }
  | { type: 'record-selection'; action: CaptureCompletionAction | 'ocr' }
  | { type: 'finish-session' };

export function getCaptureModeSelectionFlow(
  mode: CaptureMode,
): CaptureSelectionFlow {
  return getCaptureSelectionFlowForMode(mode);
}

export function getPrimaryCaptureCompletionActionForMode(
  mode: CaptureMode,
): CaptureCompletionAction {
  const flow = getCaptureModeSelectionFlow(mode);
  if (flow === 'ocr' || flow === 'silent-ocr' || flow === 'ocr-translate') {
    return flow;
  }
  return 'copy';
}

export function planCandidateSelectionCompletion(
  action: CaptureCompletionAction,
): CaptureRuntimeEffect[] {
  const effects: CaptureRuntimeEffect[] = [];

  if (isOutputCaptureAction(action)) {
    effects.push({ type: 'output-capture', action });
  } else if (action === 'ocr') {
    effects.push({ type: 'run-ocr', resultWindow: 'ocr' });
  } else if (action === 'ocr-translate') {
    effects.push({ type: 'run-ocr', resultWindow: 'translation' });
  } else if (action === 'silent-ocr') {
    effects.push({ type: 'run-ocr', resultWindow: null });
  }

  if (shouldRecordSuccessfulCaptureSelection(action)) {
    effects.push({ type: 'record-selection', action });
  }

  if (action !== 'cancel') {
    effects.push({ type: 'finish-session' });
  }

  return effects;
}

export function planSelectionFlowCompletion(
  flow: CaptureSelectionFlow,
): CaptureRuntimeEffect[] {
  if (flow === 'preview') return [];

  const resultWindow =
    flow === 'ocr-translate' ? 'translation' : flow === 'ocr' ? 'ocr' : null;

  return [
    { type: 'run-ocr', resultWindow },
    { type: 'record-selection', action: 'ocr' },
    { type: 'finish-session' },
  ];
}

function isOutputCaptureAction(
  action: CaptureCompletionAction,
): action is CaptureRuntimeOutputAction {
  return (
    action === 'copy' ||
    action === 'save' ||
    action === 'quick-save' ||
    action === 'pin' ||
    action === 'print'
  );
}
