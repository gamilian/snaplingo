type CapturePresentationStatus = 'idle' | 'loading' | 'selecting' | 'preview' | 'error';

export function getCaptureRootClassName(_status: CapturePresentationStatus) {
  return 'fixed left-0 top-0 z-[9999] cursor-crosshair select-none overflow-hidden bg-transparent text-white';
}

export function shouldShowCaptureLoadingMask(_status: CapturePresentationStatus) {
  return false;
}
