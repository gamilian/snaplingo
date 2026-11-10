import { createContext, useContext, type ReactNode } from 'react';
import type { CaptureWorkspacePorts } from '../../application/capture-workspace/ports';

const CaptureWorkspacePortsContext =
  createContext<CaptureWorkspacePorts | null>(null);

export function CaptureWorkspacePortsProvider({
  children,
  ports,
}: {
  children: ReactNode;
  ports: CaptureWorkspacePorts;
}) {
  return (
    <CaptureWorkspacePortsContext.Provider value={ports}>
      {children}
    </CaptureWorkspacePortsContext.Provider>
  );
}

export function useCaptureWorkspacePorts() {
  const ports = useContext(CaptureWorkspacePortsContext);
  if (!ports) throw new Error('Capture workspace ports are unavailable');
  return ports;
}
