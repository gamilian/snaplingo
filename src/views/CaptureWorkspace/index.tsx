import { useSettingsConfigStore } from '../../stores/settingsConfigStore';
import type { CaptureWorkspacePlatformRuntime } from '../../application/capture-workspace/platformRuntime';
import { ANNOTATION_COLORS } from './annotationStyle';
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
  const screenshotPreferences = useSettingsConfigStore((state) => state.screenshot);
  const annotationColorPresets = useSettingsConfigStore(
    (state) => state.screenshot?.annotationColors ?? ANNOTATION_COLORS,
  );
  const updateAnnotationColorPresets = useSettingsConfigStore(
    (state) => state.updateAnnotationColors,
  );
  const { renderState, actions } = useCaptureWorkspaceRuntimeView({
    initialMode,
    initialSessionId,
    onInactive,
    annotationColorPresets,
    screenshotPreferences: screenshotPreferences ?? undefined,
  });

  return (
    <CaptureWorkspaceView
      renderState={renderState}
      actions={actions}
      annotationColorPresets={annotationColorPresets}
      onUpdateAnnotationColorPresets={updateAnnotationColorPresets}
    />
  );
}
