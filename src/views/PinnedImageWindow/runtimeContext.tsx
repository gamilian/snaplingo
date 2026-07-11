import { createContext, useContext, type ReactNode } from 'react';
import type { PinnedImagePlatformRuntime } from '../../application/pinned-image/platformRuntime';

const PinnedImageRuntimeContext =
  createContext<PinnedImagePlatformRuntime | null>(null);

export function PinnedImageRuntimeProvider({
  children,
  runtime,
}: {
  children: ReactNode;
  runtime: PinnedImagePlatformRuntime;
}) {
  return (
    <PinnedImageRuntimeContext.Provider value={runtime}>
      {children}
    </PinnedImageRuntimeContext.Provider>
  );
}

export function usePinnedImageRuntime() {
  const runtime = useContext(PinnedImageRuntimeContext);
  if (!runtime) throw new Error('Pinned image runtime is unavailable');
  return runtime;
}
