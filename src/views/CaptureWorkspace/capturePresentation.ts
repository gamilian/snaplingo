type CapturePresentationStatus = 'idle' | 'loading' | 'selecting' | 'preview' | 'error';
type CaptureEditorCommandButtonVariant = 'default' | 'icon' | 'primary';

export function getCaptureRootClassName(_status: CapturePresentationStatus) {
  return [
    'fixed left-0 top-0 z-[9999] cursor-crosshair select-none overflow-hidden bg-transparent text-white',
  ].join(' ');
}

export function getCaptureRootCursorStyle(status: CapturePresentationStatus) {
  return status === 'selecting' ? 'none' : 'crosshair';
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
  _hasActiveAnnotationTool = false,
) {
  return [
    'absolute rounded-[8px] bg-transparent',
    status === 'preview'
      ? 'cursor-crosshair'
      : '',
  ]
    .filter(Boolean)
    .join(' ');
}

export function getCaptureEditorIconButtonClassName(isActive = false) {
  return [
    'flex h-7 w-7 shrink-0 items-center justify-center rounded-[8px] border leading-none',
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
