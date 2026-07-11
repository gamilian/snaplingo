import { createContext, useContext, type ReactNode } from 'react';
import type { CaptureWorkspacePlatformRuntime } from '../../application/capture-workspace/platformRuntime';

const CaptureWorkspaceRuntimeContext =
  createContext<CaptureWorkspacePlatformRuntime | null>(null);

export function CaptureWorkspaceRuntimeProvider({
  children,
  runtime,
}: {
  children: ReactNode;
  runtime: CaptureWorkspacePlatformRuntime;
}) {
  return (
    <CaptureWorkspaceRuntimeContext.Provider value={runtime}>
      {children}
    </CaptureWorkspaceRuntimeContext.Provider>
  );
}

export function useCaptureWorkspaceRuntime() {
  const runtime = useContext(CaptureWorkspaceRuntimeContext);
  if (!runtime) throw new Error('Capture workspace runtime is unavailable');
  return runtime;
}
