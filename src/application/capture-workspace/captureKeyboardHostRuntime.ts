import { shouldCancelCaptureOnBlur } from './captureActions';

type CaptureKeyboardHostStatus =
  | 'idle'
  | 'loading'
  | 'selecting'
  | 'preview'
  | 'error';

interface CaptureKeyboardKeyEvent {
  key: string;
}

interface CaptureKeyboardKeyUpState {
  hasDraftSelectionMoveGesture: boolean;
}

interface CaptureKeyboardBlurState {
  status: CaptureKeyboardHostStatus;
  isRenderingOutput: boolean;
}

export type CaptureKeyboardKeyUpAction =
  | 'release-magnifier-request'
  | 'finish-draft-selection-move';

export interface CaptureKeyboardBlurPlan {
  releaseMagnifierRequest: boolean;
  cancelSession: boolean;
}

export function getCaptureKeyboardKeyUpAction(
  event: CaptureKeyboardKeyEvent,
  state: CaptureKeyboardKeyUpState,
): CaptureKeyboardKeyUpAction | null {
  if (event.key === 'Alt') return 'release-magnifier-request';
  if (event.key === ' ' && state.hasDraftSelectionMoveGesture) {
    return 'finish-draft-selection-move';
  }

  return null;
}

export function planCaptureKeyboardBlur({
  isRenderingOutput,
  status,
}: CaptureKeyboardBlurState): CaptureKeyboardBlurPlan {
  return {
    releaseMagnifierRequest: true,
    cancelSession: shouldCancelCaptureOnBlur({
      status,
      isRenderingOutput,
    }),
  };
}
