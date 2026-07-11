import { useEffect, type RefObject } from 'react';
import {
  getCaptureKeyboardKeyUpAction,
  planCaptureKeyboardBlur,
} from './captureKeyboardHostRuntime';

type CaptureKeyboardHostStatus =
  | 'idle'
  | 'loading'
  | 'selecting'
  | 'preview'
  | 'error';

interface UseCaptureKeyboardHostEventsOptions {
  isActive: boolean;
  status: CaptureKeyboardHostStatus;
  isRenderingOutputRef: RefObject<boolean>;
  hasDraftSelectionMoveGesture: boolean;
  onKeyDown: (event: KeyboardEvent) => void;
  onReleaseMagnifierRequest: () => void;
  onFinishDraftSelectionMove: () => void;
  onCancelSession: () => void | Promise<void>;
}

export function useCaptureKeyboardHostEvents({
  hasDraftSelectionMoveGesture,
  isActive,
  isRenderingOutputRef,
  onCancelSession,
  onFinishDraftSelectionMove,
  onKeyDown,
  onReleaseMagnifierRequest,
  status,
}: UseCaptureKeyboardHostEventsOptions) {
  useEffect(() => {
    if (!isActive) return;

    const handleKeyUp = (event: KeyboardEvent) => {
      const action = getCaptureKeyboardKeyUpAction(event, {
        hasDraftSelectionMoveGesture,
      });

      if (action === 'release-magnifier-request') {
        onReleaseMagnifierRequest();
        return;
      }

      if (action === 'finish-draft-selection-move') {
        event.preventDefault();
        onFinishDraftSelectionMove();
      }
    };

    const handleWindowBlur = () => {
      const plan = planCaptureKeyboardBlur({
        status,
        isRenderingOutput: isRenderingOutputRef.current ?? false,
      });

      if (plan.releaseMagnifierRequest) {
        onReleaseMagnifierRequest();
      }
      if (plan.cancelSession) {
        void onCancelSession();
      }
    };

    window.addEventListener('keydown', onKeyDown);
    window.addEventListener('keyup', handleKeyUp);
    window.addEventListener('blur', handleWindowBlur);
    return () => {
      window.removeEventListener('keydown', onKeyDown);
      window.removeEventListener('keyup', handleKeyUp);
      window.removeEventListener('blur', handleWindowBlur);
    };
  }, [
    hasDraftSelectionMoveGesture,
    isActive,
    isRenderingOutputRef,
    onCancelSession,
    onFinishDraftSelectionMove,
    onKeyDown,
    onReleaseMagnifierRequest,
    status,
  ]);
}
