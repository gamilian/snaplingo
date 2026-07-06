import { useEffect, type MutableRefObject } from 'react';
import {
  revealCaptureHostWindow,
  subscribeCaptureCancelHostRequests,
  subscribeCaptureCopyHostRequests,
  subscribeCaptureHotkeyLaunches,
  type CaptureHotkeyLaunchListener,
} from './captureHostRuntime';
import type { CaptureWindowHandle } from './captureWindowVisibility';

type CaptureHostRevealStatus = 'idle' | 'loading' | 'selecting' | 'preview' | 'error';

interface UseCaptureHostSubscriptionsOptions {
  isActive: boolean;
  onLaunch: CaptureHotkeyLaunchListener;
  onCancel: () => void | Promise<void>;
  onCopy: () => void | Promise<void>;
}

interface UseCaptureHostWindowRevealOptions {
  status: CaptureHostRevealStatus;
  sessionId: string | null;
  hasCaptureImagesReady: boolean;
  hasRevealedRef: MutableRefObject<boolean>;
  window: CaptureWindowHandle;
  prepareSurface: () => void | Promise<void>;
  onRevealedSession: (sessionId: string) => void;
  onError: (err: unknown) => void;
}

export function useCaptureHostWindowReveal({
  hasCaptureImagesReady,
  hasRevealedRef,
  onError,
  onRevealedSession,
  prepareSurface,
  sessionId,
  status,
  window,
}: UseCaptureHostWindowRevealOptions) {
  useEffect(() => {
    void revealCaptureHostWindow({
      status,
      sessionId,
      hasCaptureImagesReady,
      hasRevealed: hasRevealedRef.current,
      window,
      prepareSurface,
    })
      .then((didReveal) => {
        if (!didReveal) return;
        hasRevealedRef.current = true;
        if (sessionId) {
          onRevealedSession(sessionId);
        }
      })
      .catch(onError);
  }, [
    hasCaptureImagesReady,
    hasRevealedRef,
    onError,
    onRevealedSession,
    prepareSurface,
    sessionId,
    status,
    window,
  ]);
}

export function useCaptureHostSubscriptions({
  isActive,
  onCancel,
  onCopy,
  onLaunch,
}: UseCaptureHostSubscriptionsOptions) {
  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;

    subscribeCaptureHotkeyLaunches(onLaunch)
      .then((nextUnlisten) => {
        if (disposed) {
          nextUnlisten();
        } else {
          unlisten = nextUnlisten;
        }
      })
      .catch((err) => {
        console.error('Failed to listen for capture hotkeys:', err);
      });

    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [onLaunch]);

  useEffect(() => {
    if (!isActive) return;

    let disposed = false;
    let unlisten: (() => void) | undefined;

    subscribeCaptureCancelHostRequests(onCancel)
      .then((nextUnlisten) => {
        if (disposed) {
          nextUnlisten();
        } else {
          unlisten = nextUnlisten;
        }
      })
      .catch((err) => {
        console.error('Failed to listen for native capture cancel requests:', err);
      });

    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [isActive, onCancel]);

  useEffect(() => {
    if (!isActive) return;

    let disposed = false;
    let unlisten: (() => void) | undefined;

    subscribeCaptureCopyHostRequests(onCopy)
      .then((nextUnlisten) => {
        if (disposed) {
          nextUnlisten();
        } else {
          unlisten = nextUnlisten;
        }
      })
      .catch((err) => {
        console.error('Failed to listen for native capture copy requests:', err);
      });

    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [isActive, onCopy]);
}
