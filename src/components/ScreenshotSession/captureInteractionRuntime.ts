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

export type CaptureRuntimeOcrTarget =
  | 'ocr-window'
  | 'translation-window'
  | 'clipboard';

export type CaptureRuntimeEffect =
  | { type: 'output-capture'; action: CaptureRuntimeOutputAction }
  | { type: 'run-ocr'; target: CaptureRuntimeOcrTarget }
  | { type: 'record-selection'; action: CaptureCompletionAction | 'ocr' }
  | { type: 'finish-session' };

export type ManualSelectionCompletionPlan =
  | { type: 'preview' }
  | { type: 'effects'; effects: CaptureRuntimeEffect[] };

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
    effects.push({ type: 'run-ocr', target: 'ocr-window' });
  } else if (action === 'ocr-translate') {
    effects.push({ type: 'run-ocr', target: 'translation-window' });
  } else if (action === 'silent-ocr') {
    effects.push({ type: 'run-ocr', target: 'clipboard' });
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
  if (flow === 'copy') return planCandidateSelectionCompletion('copy');

  const target =
    flow === 'ocr-translate'
      ? 'translation-window'
      : flow === 'ocr'
        ? 'ocr-window'
        : 'clipboard';

  return [
    { type: 'run-ocr', target },
    { type: 'record-selection', action: 'ocr' },
    { type: 'finish-session' },
  ];
}

export function planManualSelectionCompletion(
  mode: CaptureMode,
): ManualSelectionCompletionPlan {
  const flow = getCaptureModeSelectionFlow(mode);
  if (flow === 'preview') return { type: 'preview' };

  return {
    type: 'effects',
    effects: planSelectionFlowCompletion(flow),
  };
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
