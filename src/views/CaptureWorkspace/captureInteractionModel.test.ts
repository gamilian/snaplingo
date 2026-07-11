import { describe, expect, it } from 'vitest';

import {
  getCaptureCompletionPlan,
  getCaptureModeSelectionFlow,
  getPrimaryCaptureCompletionActionForMode,
  shouldRecordSuccessfulCaptureCompletion,
  type CaptureCompletionEffect,
} from './captureInteractionModel';
import type { CaptureCompletionAction } from './captureActions';

describe('capture interaction model', () => {
  it('records only successful screenshot output actions', () => {
    expect(recordedActions()).toEqual(['copy', 'save', 'quick-save', 'pin']);
  });

  it('chooses the selection flow from the capture mode', () => {
    expect(getCaptureModeSelectionFlow('screenshot')).toBe('preview');
    expect(getCaptureModeSelectionFlow('screenshot-copy')).toBe('copy');
    expect(getCaptureModeSelectionFlow('screenshot-ocr')).toBe('ocr');
    expect(getCaptureModeSelectionFlow('silent-screenshot-ocr')).toBe('silent-ocr');
    expect(getCaptureModeSelectionFlow('screenshot-translate')).toBe('ocr-translate');
  });

  it('chooses the primary completion action from the capture mode', () => {
    expect(getPrimaryCaptureCompletionActionForMode('screenshot')).toBe('copy');
    expect(getPrimaryCaptureCompletionActionForMode('screenshot-copy')).toBe(
      'copy',
    );
    expect(getPrimaryCaptureCompletionActionForMode('screenshot-ocr')).toBe('ocr');
    expect(getPrimaryCaptureCompletionActionForMode('silent-screenshot-ocr')).toBe(
      'silent-ocr',
    );
    expect(getPrimaryCaptureCompletionActionForMode('screenshot-translate')).toBe(
      'ocr-translate',
    );
  });

  it('plans the side-effect lane for every completion action', () => {
    expect(effectByAction()).toEqual({
      copy: 'copy',
      save: 'save',
      'quick-save': 'quick-save',
      pin: 'pin',
      ocr: 'ocr',
      'silent-ocr': 'ocr',
      'ocr-translate': 'ocr',
      print: 'print',
      cancel: 'cancel',
    });
  });

  it('plans OCR targets and session completion', () => {
    expect(getCaptureCompletionPlan('ocr')).toMatchObject({
      ocrTarget: 'ocr-window',
      shouldFinishSession: true,
    });
    expect(getCaptureCompletionPlan('ocr-translate')).toMatchObject({
      ocrTarget: 'translation-window',
      shouldFinishSession: true,
    });
    expect(getCaptureCompletionPlan('silent-ocr')).toMatchObject({
      ocrTarget: 'clipboard',
      shouldFinishSession: true,
    });
    expect(getCaptureCompletionPlan('cancel')).toMatchObject({
      ocrTarget: null,
      shouldFinishSession: false,
    });
  });
});

const actions: CaptureCompletionAction[] = [
  'copy',
  'save',
  'quick-save',
  'pin',
  'ocr',
  'silent-ocr',
  'ocr-translate',
  'print',
  'cancel',
];

function recordedActions() {
  return actions.filter(shouldRecordSuccessfulCaptureCompletion);
}

function effectByAction(): Record<CaptureCompletionAction, CaptureCompletionEffect> {
  return Object.fromEntries(
    actions.map((action) => [action, getCaptureCompletionPlan(action).effect]),
  ) as Record<CaptureCompletionAction, CaptureCompletionEffect>;
}
