import type { MonitorSnapshotView } from '../../domain/capture';

export function captureMonitorImageSource(
  monitor?: MonitorSnapshotView | null,
): string | null {
  return monitor?.image_url || (
    monitor?.image_base64 ? `data:image/png;base64,${monitor.image_base64}` : null
  );
}
