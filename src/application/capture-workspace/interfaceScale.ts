import type { CaptureUiScale } from '../settings/ports';
import type { LogicalRect } from '../../domain/capture';

export function captureInterfaceScale(
  mode: CaptureUiScale | undefined,
  display?: LogicalRect | null,
) {
  if (mode && mode !== 'auto') return Number(mode) / 100;
  if (!display) return 1;
  // WebViews already apply the OS display scale to CSS pixels.
  return Math.min(
    3,
    Math.max(1, display.width / 1920, display.height / 1080) ** 0.6,
  );
}
