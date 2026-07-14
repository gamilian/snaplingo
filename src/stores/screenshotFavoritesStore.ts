import { create } from 'zustand';
import type { ScreenshotFavoriteItem } from '../application/settings/ports';
import type { SettingsRuntime } from '../application/settings/runtime';

type Runtime = SettingsRuntime['screenshotFavorites'];

let configuredRuntime: Runtime | null = null;

export function initializeScreenshotFavoritesStore(runtime: Runtime) {
  configuredRuntime = runtime;
}

function runtime() {
  if (!configuredRuntime) {
    throw new Error('Screenshot favorites store runtime has not been initialized');
  }
  return configuredRuntime;
}

interface ScreenshotFavoritesState {
  items: ScreenshotFavoriteItem[];
  total: number;
  loading: boolean;
  error: string | null;
  revision: number;
  invalidate(): void;
  query(search: string, limit: number, offset: number): Promise<void>;
  updateMetadata(
    id: number,
    note: string | null,
    tags: string[],
  ): Promise<void>;
  delete(id: number): Promise<void>;
  copy(id: number): Promise<void>;
  reveal(id: number): Promise<void>;
}

export const useScreenshotFavoritesStore = create<ScreenshotFavoritesState>(
  (set) => ({
    items: [],
    total: 0,
    loading: false,
    error: null,
    revision: 0,
    invalidate: () => set((state) => ({ revision: state.revision + 1 })),
    query: async (search, limit, offset) => {
      set({ loading: true, error: null });
      try {
        const page = await runtime().query({ search, limit, offset });
        set({ items: page.items, total: page.total, loading: false });
      } catch (error) {
        set({
          loading: false,
          error: error instanceof Error ? error.message : String(error),
        });
      }
    },
    updateMetadata: async (id, note, tags) => {
      await runtime().updateMetadata(id, note, tags);
      set((state) => ({
        items: state.items.map((item) =>
          item.id === id ? { ...item, note, tags } : item,
        ),
      }));
    },
    delete: async (id) => {
      await runtime().delete(id);
      set((state) => ({
        items: state.items.filter((item) => item.id !== id),
        total: Math.max(0, state.total - 1),
      }));
    },
    copy: (id) => runtime().copy(id),
    reveal: (id) => runtime().reveal(id),
  }),
);
