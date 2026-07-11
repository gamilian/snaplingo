import { createContext, useContext, type ReactNode } from 'react';
import type { ResultWindowPlatformRuntime } from '../../application/result-window/platformRuntime';

const ResultWindowRuntimeContext =
  createContext<ResultWindowPlatformRuntime | null>(null);

export function ResultWindowRuntimeProvider({
  children,
  runtime,
}: {
  children: ReactNode;
  runtime: ResultWindowPlatformRuntime;
}) {
  return (
    <ResultWindowRuntimeContext.Provider value={runtime}>
      {children}
    </ResultWindowRuntimeContext.Provider>
  );
}

export function useResultWindowRuntime() {
  const runtime = useContext(ResultWindowRuntimeContext);
  if (!runtime) throw new Error('Result window runtime is unavailable');
  return runtime;
}
