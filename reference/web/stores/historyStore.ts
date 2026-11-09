import { create } from 'zustand';

interface HistoryState {
  revision: number;
  invalidate(): void;
}

export const useHistoryStore = create<HistoryState>((set) => ({
  revision: 0,
  invalidate: () => set((state) => ({ revision: state.revision + 1 })),
}));
