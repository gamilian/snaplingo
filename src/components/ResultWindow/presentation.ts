export type ResultWindowPresentation = 'overlay' | 'standalone';

export function resultWindowContainerClassName(
  presentation: ResultWindowPresentation,
) {
  if (presentation === 'standalone') {
    return 'min-h-screen bg-white flex items-stretch justify-stretch';
  }

  return 'fixed inset-0 bg-black/25 backdrop-blur-sm flex items-center justify-center z-50 p-8';
}

export function resultWindowPanelClassName(
  presentation: ResultWindowPresentation,
) {
  if (presentation === 'standalone') {
    return 'bg-white w-full h-screen overflow-hidden flex flex-col';
  }

  return 'bg-white rounded-2xl shadow-xl w-full max-w-3xl max-h-[90vh] overflow-hidden flex flex-col animate-[slideIn_0.3s_ease-out]';
}

export function shouldCloseFromContainerClick(
  presentation: ResultWindowPresentation,
  target: EventTarget,
  currentTarget: EventTarget,
) {
  return presentation === 'overlay' && target === currentTarget;
}
