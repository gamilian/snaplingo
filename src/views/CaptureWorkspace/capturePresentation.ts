import {
  annotationBrushDiameter,
  type AnnotationTool,
} from './annotationStyle';

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
  if (status === 'selecting') return 'none';
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

function cursorDataUrl(svg: string, hotspotX: number, hotspotY: number) {
  return `url("data:image/svg+xml,${encodeURIComponent(svg)}") ${hotspotX} ${hotspotY}, auto`;
}

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
    'absolute flex h-[42px] items-center gap-1 rounded-[12px] bg-white/95 px-1.5 py-1',
    'text-[11px] font-semibold text-slate-600 shadow-[0_10px_28px_rgba(15,23,42,0.2)]',
    'ring-1 ring-slate-200/90 backdrop-blur-xl',
  ].join(' ');
}

export function getCaptureEditorSelectionClassName(
  status: CapturePresentationStatus,
  activeAnnotationTool: AnnotationTool | null = null,
) {
  return [
    'absolute rounded-[8px] bg-transparent',
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
    `flex h-7 ${widthClassName} shrink-0 items-center justify-center rounded-[8px] border leading-none`,
    'transition-colors disabled:cursor-not-allowed disabled:opacity-40',
    isActive
      ? 'border-[#5b7fff] bg-[#eef3ff] text-[#4a6fe8] shadow-[0_0_0_2px_rgba(91,127,255,0.2)]'
      : 'border-transparent bg-transparent text-slate-600 hover:bg-slate-100',
  ].join(' ');
}

export function getCaptureEditorCommandButtonClassName(
  variant: CaptureEditorCommandButtonVariant = 'default',
) {
  const base =
    'flex h-7 shrink-0 items-center justify-center rounded-[8px] text-[11px] font-semibold transition-colors disabled:cursor-not-allowed disabled:opacity-40';

  if (variant === 'primary') {
    return `${base} w-7 bg-[#5b7fff] text-white shadow-[0_5px_12px_rgba(91,127,255,0.28)] hover:bg-[#4a6fe8]`;
  }

  if (variant === 'icon') {
    return `${base} w-7 border border-slate-200 bg-white text-slate-600 hover:bg-slate-50`;
  }

  return `${base} border border-slate-200 bg-white px-2 text-slate-600 hover:bg-slate-50`;
}

export function getCaptureEditorDividerClassName() {
  return 'mx-0.5 h-6 w-px shrink-0 bg-slate-200';
}
