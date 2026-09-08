import { useSettingsConfigStore } from '../../stores/settingsConfigStore';
import type { CaptureWorkspacePorts } from '../../application/capture-workspace/ports';
import { ANNOTATION_COLORS } from '../../application/capture-workspace/annotationStyle';
import { CaptureWorkspaceView } from './CaptureWorkspaceView';
import { useCaptureWorkspaceRuntimeView } from './useCaptureWorkspaceRuntimeView';
import type { CaptureMode } from './types';
import {
  CaptureWorkspacePortsProvider,
} from './runtimeContext';

interface CaptureWorkspaceProps {
  initialMode?: CaptureMode;
  initialSessionId?: string;
  onInactive?: () => void | Promise<void>;
  ports: CaptureWorkspacePorts;
}

export default function CaptureWorkspace({ ports, ...props }: CaptureWorkspaceProps) {
  return (
    <CaptureWorkspacePortsProvider ports={ports}>
      <CaptureWorkspaceContent {...props} />
    </CaptureWorkspacePortsProvider>
  );
}

function CaptureWorkspaceContent({
  initialMode,
  initialSessionId,
  onInactive,
}: Omit<CaptureWorkspaceProps, 'ports'>) {
  const screenshotPreferences = useSettingsConfigStore((state) => state.screenshot);
  const ocrPreferences = useSettingsConfigStore((state) => state.ocr);
  const annotationColorPresets = useSettingsConfigStore(
    (state) => state.screenshot?.annotationColors ?? ANNOTATION_COLORS,
  );
  const updateAnnotationColorPresets = useSettingsConfigStore(
    (state) => state.updateAnnotationColors,
  );
  const updateScreenshotSettings = useSettingsConfigStore(
    (state) => state.updateScreenshotSettings,
  );
  const { renderState, actions } = useCaptureWorkspaceRuntimeView({
    initialMode,
    initialSessionId,
    onInactive,
    annotationColorPresets,
    screenshotPreferences: screenshotPreferences ?? undefined,
    persistScreenshotDefaults: (input) => {
      void updateScreenshotSettings(input);
    },
    ocrPreferences: ocrPreferences ?? undefined,
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
