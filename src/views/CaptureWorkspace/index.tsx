import { useSettingsConfigStore } from '../../stores/settingsConfigStore';
import type { CaptureWorkspacePlatformRuntime } from '../../application/capture-workspace/platformRuntime';
import { CaptureWorkspaceView } from './CaptureWorkspaceView';
import { useCaptureWorkspaceRuntimeView } from './useCaptureWorkspaceRuntimeView';
import type { CaptureMode } from './types';
import {
  CaptureWorkspaceRuntimeProvider,
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
  const screenshotSavePath = useSettingsConfigStore(
    (state) => state.screenshot?.savePath,
  );
  const { renderState, actions } = useCaptureWorkspaceRuntimeView({
    initialMode,
    initialSessionId,
    onInactive,
    screenshotSavePath,
  });

  return <CaptureWorkspaceView renderState={renderState} actions={actions} />;
}
