import {
  annotationBrushDiameter,
  type AnnotationTool,
} from '../../application/capture-workspace/annotationStyle';

type CapturePresentationStatus = 'idle' | 'loading' | 'selecting' | 'preview' | 'error';
type CaptureEditorCommandButtonVariant = 'default' | 'icon' | 'primary';

export function getCaptureRootClassName(_status: CapturePresentationStatus) {
  return [
    'fixed left-0 top-0 z-[9999] cursor-crosshair select-none overflow-hidden bg-transparent text-white',
  ].join(' ');
}

export function getCaptureRootCursorStyle(
  status: CapturePresentationStatus,
  activeAnnotationTool: AnnotationTool | null = null,
  strokeWidth = 2,
) {
  if (status === 'selecting') return SELECTION_CROSSHAIR_CURSOR;
  return getCaptureEditorCursorStyle(activeAnnotationTool, strokeWidth);
}

export function getCaptureEditorCursorStyle(
  activeAnnotationTool: AnnotationTool | null,
  strokeWidth = 2,
) {
  if (activeAnnotationTool === 'text') return 'text';
  if (activeAnnotationTool === 'mosaic') return brushCursor('mosaic', strokeWidth);
  if (activeAnnotationTool === 'eraser') return brushCursor('eraser', strokeWidth);
  if (activeAnnotationTool === 'pen' || activeAnnotationTool === 'highlight') {
    return pencilCursor();
  }
  return 'default';
}

function cursorDataUrl(
  svg: string,
  hotspotX: number,
  hotspotY: number,
  fallback = 'auto',
) {
  return `url("data:image/svg+xml,${encodeURIComponent(svg)}") ${hotspotX} ${hotspotY}, ${fallback}`;
}

const SELECTION_CROSSHAIR_CURSOR = cursorDataUrl(
  '<svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 20 20"><path d="M10 3v5M10 12v5M3 10h5M12 10h5" fill="none" stroke="rgba(15,23,42,.55)" stroke-width="2" stroke-linecap="round"/><path d="M10 3v5M10 12v5M3 10h5M12 10h5" fill="none" stroke="rgba(255,255,255,.94)" stroke-width=".75" stroke-linecap="round"/><circle cx="10" cy="10" r="1.15" fill="rgba(255,255,255,.96)" stroke="rgba(15,23,42,.55)" stroke-width=".65"/></svg>',
  10,
  10,
  'crosshair',
);

function pencilCursor() {
  return cursorDataUrl(
    '<svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 20 20"><path d="m3 14.8 9.8-9.8 3 3L6 17.8H3v-3Z" fill="white" stroke="black" stroke-width="1.2"/><path d="m12.8 5 1.2-1.2a1.6 1.6 0 0 1 2.3 2.3L15 7.3" fill="#f5c98a" stroke="black" stroke-width="1.2"/><path d="m3 18 2.5-.8-1.7-1.7L3 18Z" fill="#222"/></svg>',
    2,
    18,
  );
}

function brushCursor(
  tool: Extract<AnnotationTool, 'mosaic' | 'eraser'>,
  strokeWidth: number,
) {
  const diameter = annotationBrushDiameter(tool, strokeWidth);
  const radius = diameter / 2;
  return cursorDataUrl(
    `<svg xmlns="http://www.w3.org/2000/svg" width="${diameter}" height="${diameter}" viewBox="0 0 ${diameter} ${diameter}"><circle cx="${radius}" cy="${radius}" r="${radius - 1}" fill="rgba(107,114,128,.28)" stroke="#6b7280" stroke-width="1.25"/></svg>`,
    radius,
    radius,
  );
}

export function shouldShowCaptureLoadingMask(_status: CapturePresentationStatus) {
  return false;
}

export function getCaptureSelectionOverlayCanvasClassName() {
  return 'pointer-events-none absolute left-0 top-0 h-full w-full';
}

export function getCaptureEditorToolbarClassName() {
  return [
    'absolute flex h-[40px] items-center gap-0.5 rounded-[11px] bg-surface px-[5px] py-1',
    'text-hint font-semibold text-muted shadow-[0_10px_30px_rgba(0,0,0,0.34)]',
    'ring-1 ring-line',
  ].join(' ');
}

export function getCaptureEditorSelectionClassName(
  status: CapturePresentationStatus,
  activeAnnotationTool: AnnotationTool | null = null,
) {
  return [
    'absolute rounded-md bg-transparent',
    status === 'preview'
      ? getCaptureEditorCursorClassName(activeAnnotationTool)
      : '',
  ]
    .filter(Boolean)
    .join(' ');
}

function getCaptureEditorCursorClassName(
  activeAnnotationTool: AnnotationTool | null,
) {
  if (activeAnnotationTool === 'text') return 'cursor-text';
  if (activeAnnotationTool === 'mosaic' || activeAnnotationTool === 'eraser') {
    return 'cursor-crosshair';
  }
  return 'cursor-default';
}

export function getCaptureEditorIconButtonClassName(
  isActive = false,
  widthClassName = 'w-7',
) {
  return [
    `flex h-7 ${widthClassName} shrink-0 items-center justify-center rounded-sm leading-none`,
    'transition-colors disabled:cursor-not-allowed disabled:opacity-40',
    isActive
      ? 'bg-accent-soft text-accent-text shadow-[inset_0_0_0_1px_rgb(var(--accent-line))]'
      : 'text-muted hover:bg-raised hover:text-ink',
  ].join(' ');
}

export function getCaptureEditorCommandButtonClassName(
  variant: CaptureEditorCommandButtonVariant = 'default',
) {
  const base =
    'flex h-7 shrink-0 items-center justify-center rounded-sm text-hint font-semibold transition-colors disabled:cursor-not-allowed disabled:opacity-40';

  if (variant === 'primary') {
    return `${base} w-7 bg-accent text-accent-on hover:bg-accent-hover`;
  }

  if (variant === 'icon') {
    return `${base} w-7 text-muted hover:bg-raised hover:text-ink`;
  }

  return `${base} px-2 text-muted hover:bg-raised hover:text-ink`;
}

export function getCaptureEditorDividerClassName() {
  return 'mx-1 h-[18px] w-px shrink-0 bg-line';
}
