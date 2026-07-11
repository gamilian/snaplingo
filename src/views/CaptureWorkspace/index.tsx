import { useSettingsConfigStore } from '../../stores/settingsConfigStore';
import type { CaptureWorkspacePlatformRuntime } from '../../application/capture-workspace/platformRuntime';
import {
  useCaptureHostSubscriptions,
  useCaptureHostWindowReveal,
} from './captureHostRuntimeHooks';
import { useCaptureKeyboardHostEvents } from './captureKeyboardHostRuntimeHooks';
import { CaptureWorkspaceView } from './CaptureWorkspaceView';
import { useCaptureWorkspaceController } from './useCaptureWorkspaceController';
import type { CaptureMode } from './types';
import {
  CaptureWorkspaceRuntimeProvider,
  useCaptureWorkspaceRuntime,
} from './runtimeContext';

interface CaptureWorkspaceProps {
  initialMode?: CaptureMode;
  initialSessionId?: string;
  onInactive?: () => void | Promise<void>;
  runtime: CaptureWorkspacePlatformRuntime;
}

export default function CaptureWorkspace({ runtime, ...props }: CaptureWorkspaceProps) {
  return (
    <CaptureWorkspaceRuntimeProvider runtime={runtime}>
      <CaptureWorkspaceContent {...props} />
    </CaptureWorkspaceRuntimeProvider>
  );
}

function CaptureWorkspaceContent({
  initialMode,
  initialSessionId,
  onInactive,
}: Omit<CaptureWorkspaceProps, 'runtime'>) {
  const runtime = useCaptureWorkspaceRuntime();
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
    window: {
      show: runtime.reveal,
      setFocus: async () => undefined,
    },
  });
  useCaptureHostSubscriptions(controller.hostSubscriptions);
  useCaptureKeyboardHostEvents(controller.keyboardHostEvents);

  return <CaptureWorkspaceView {...controller.viewProps} />;
}
