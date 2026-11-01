import { create } from 'zustand';

interface ScreenshotFavoritesState {
  revision: number;
  invalidate(): void;
}

export const useScreenshotFavoritesStore = create<ScreenshotFavoritesState>((set) => ({
  revision: 0,
  invalidate: () => set((state) => ({ revision: state.revision + 1 })),
}));
