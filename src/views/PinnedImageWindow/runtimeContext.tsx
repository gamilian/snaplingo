import { createContext, useContext, type ReactNode } from 'react';
import type { PinnedImageRuntime } from '../../application/pinned-image/runtime';

const PinnedImageRuntimeContext =
  createContext<PinnedImageRuntime | null>(null);

export function PinnedImageRuntimeProvider({
  children,
  runtime,
}: {
  children: ReactNode;
  runtime: PinnedImageRuntime;
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
