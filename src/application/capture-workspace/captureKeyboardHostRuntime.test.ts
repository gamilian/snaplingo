import { describe, expect, it } from 'vitest';
import {
  getCaptureKeyboardKeyUpAction,
  planCaptureKeyboardBlur,
} from './captureKeyboardHostRuntime';

describe('captureKeyboardHostRuntime', () => {
  it('releases the magnifier request when Alt is released', () => {
    expect(
      getCaptureKeyboardKeyUpAction(
        { key: 'Alt' },
        { hasDraftSelectionMoveGesture: false },
      ),
    ).toBe('release-magnifier-request');
  });

  it('finishes draft-selection move only when the space gesture is active', () => {
    expect(
      getCaptureKeyboardKeyUpAction(
        { key: ' ' },
        { hasDraftSelectionMoveGesture: true },
      ),
    ).toBe('finish-draft-selection-move');

    expect(
      getCaptureKeyboardKeyUpAction(
        { key: ' ' },
        { hasDraftSelectionMoveGesture: false },
      ),
    ).toBeNull();
  });

  it('releases magnifier state on blur and cancels active capture sessions only when safe', () => {
    expect(
      planCaptureKeyboardBlur({
        status: 'selecting',
        isRenderingOutput: false,
      }),
    ).toEqual({
      releaseMagnifierRequest: true,
      cancelSession: true,
    });

    expect(
      planCaptureKeyboardBlur({
        status: 'preview',
        isRenderingOutput: true,
      }),
    ).toEqual({
      releaseMagnifierRequest: true,
      cancelSession: false,
    });

    expect(
      planCaptureKeyboardBlur({
        status: 'idle',
        isRenderingOutput: false,
      }),
    ).toEqual({
      releaseMagnifierRequest: true,
      cancelSession: false,
    });
  });
});
