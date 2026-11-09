import { createContext, useContext, type ReactNode } from 'react';
import type { ResultWindowRuntime } from '../../application/result-window/runtime';

const ResultWindowRuntimeContext =
  createContext<ResultWindowRuntime | null>(null);

export function ResultWindowRuntimeProvider({
  children,
  runtime,
}: {
  children: ReactNode;
  runtime: ResultWindowRuntime;
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
