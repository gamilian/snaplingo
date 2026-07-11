import { createContext, useContext, type ReactNode } from 'react';
import type { SettingsRuntime } from '../../application/settings/runtime';

const SettingsRuntimeContext = createContext<SettingsRuntime | null>(null);

export function SettingsRuntimeProvider({
  children,
  runtime,
}: {
  children: ReactNode;
  runtime: SettingsRuntime;
}) {
  return (
    <SettingsRuntimeContext.Provider value={runtime}>
      {children}
    </SettingsRuntimeContext.Provider>
  );
}

export function useSettingsRuntime() {
  const runtime = useContext(SettingsRuntimeContext);
  if (!runtime) throw new Error('Settings runtime is unavailable');
  return runtime;
}
