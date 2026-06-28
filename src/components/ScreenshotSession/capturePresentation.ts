type CapturePresentationStatus = 'idle' | 'loading' | 'selecting' | 'preview' | 'error';
type CaptureEditorCommandButtonVariant = 'default' | 'primary';

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
    'absolute flex h-14 items-center gap-2 rounded-[16px] bg-white/95 px-3 py-2',
    'text-sm font-semibold text-slate-600 shadow-[0_12px_34px_rgba(15,23,42,0.22)]',
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
    'flex h-9 w-9 shrink-0 items-center justify-center rounded-[10px] border text-lg leading-none',
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
    'flex h-9 shrink-0 items-center justify-center rounded-[10px] px-5 text-sm font-semibold transition-colors disabled:cursor-not-allowed disabled:opacity-40';

  if (variant === 'primary') {
    return `${base} bg-[#5b7fff] text-white shadow-[0_8px_18px_rgba(91,127,255,0.3)] hover:bg-[#4a6fe8]`;
  }

  return `${base} border border-slate-200 bg-white text-slate-600 hover:bg-slate-50`;
}

export function getCaptureEditorDividerClassName() {
  return 'h-8 w-px shrink-0 bg-slate-200';
}
