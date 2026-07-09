import { useSettingsConfigStore } from '../../stores/settingsConfigStore';
import { getCurrentAppWebviewWindow } from '../../tauri/window';
import {
  useCaptureHostSubscriptions,
  useCaptureHostWindowReveal,
} from './captureHostRuntimeHooks';
import { useCaptureKeyboardHostEvents } from './captureKeyboardHostRuntimeHooks';
import { CaptureWorkspaceView } from './CaptureWorkspaceView';
import { useCaptureWorkspaceController } from './useCaptureWorkspaceController';
import type { CaptureMode } from './types';

const captureWindow = getCurrentAppWebviewWindow();

interface ScreenshotSessionProps {
  initialMode?: CaptureMode;
  initialSessionId?: string;
  onInactive?: () => void | Promise<void>;
}

export default function ScreenshotSession({
  initialMode,
  initialSessionId,
  onInactive,
}: ScreenshotSessionProps) {
  const screenshotSavePath = useSettingsConfigStore(
    (state) => state.screenshot?.savePath,
  );
  const controller = useCaptureWorkspaceController({
    initialMode,
    initialSessionId,
    onInactive,
    screenshotSavePath,
  });

  useCaptureHostWindowReveal({
    ...controller.hostWindowReveal,
    window: captureWindow,
  });
  useCaptureHostSubscriptions(controller.hostSubscriptions);
  useCaptureKeyboardHostEvents(controller.keyboardHostEvents);

  return <CaptureWorkspaceView {...controller.viewProps} />;
}
