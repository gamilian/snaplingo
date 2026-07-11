import { useEffect, type MutableRefObject } from 'react';
import {
  revealCaptureHostWindow,
  type CaptureHotkeyLaunchListener,
} from './captureHostRuntime';
import { shouldRevealCaptureWindow } from './captureWindowVisibility';
import { useCaptureWorkspaceRuntime } from './runtimeContext';

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
}: UseCaptureHostWindowRevealOptions) {
  const runtime = useCaptureWorkspaceRuntime();
  useEffect(() => {
    void revealCaptureHostWindow({
      status,
      sessionId,
      hasCaptureImagesReady,
      hasRevealed: hasRevealedRef.current,
      prepareSurface,
    }, {
      shouldRevealCaptureWindow,
      revealCaptureWindow: () => runtime.reveal(),
      revealCaptureWindowForSession: async ({ prepareSurface }) => {
        await runtime.prepareForReveal();
        await prepareSurface?.();
        await runtime.reveal();
      },
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
    runtime,
  ]);
}

export function useCaptureHostSubscriptions({
  isActive,
  onCancel,
  onCopy,
  onLaunch,
}: UseCaptureHostSubscriptionsOptions) {
  const runtime = useCaptureWorkspaceRuntime();
  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;

    runtime.onHotkeyTriggered(onLaunch)
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
  }, [onLaunch, runtime]);

  useEffect(() => {
    if (!isActive) return;

    let disposed = false;
    let unlisten: (() => void) | undefined;

    runtime.onCancelRequested(onCancel)
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
  }, [isActive, onCancel, runtime]);

  useEffect(() => {
    if (!isActive) return;

    let disposed = false;
    let unlisten: (() => void) | undefined;

    runtime.onCopyRequested(onCopy)
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
  }, [isActive, onCopy, runtime]);
}
