import { describe, expect, it } from 'vitest';
import {
  planCandidateSelectionCompletion,
  planSelectionFlowCompletion,
} from './captureInteractionRuntime';

describe('captureInteractionRuntime', () => {
  it('plans OCR translation candidate completion as ordered runtime effects', () => {
    expect(planCandidateSelectionCompletion('ocr-translate')).toEqual([
      { type: 'run-ocr', resultWindow: 'translation' },
      { type: 'finish-session' },
    ]);
  });

  it('plans copy candidate completion with output, record, and finish effects', () => {
    expect(planCandidateSelectionCompletion('copy')).toEqual([
      { type: 'output-capture', action: 'copy' },
      { type: 'record-selection', action: 'copy' },
      { type: 'finish-session' },
    ]);
  });

  it('plans silent OCR selection flow as OCR copy text and finish effects', () => {
    expect(planSelectionFlowCompletion('silent-ocr')).toEqual([
      { type: 'run-ocr', resultWindow: null },
      { type: 'record-selection', action: 'ocr' },
      { type: 'finish-session' },
    ]);
  });
});
