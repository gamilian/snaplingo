import { create } from 'zustand';
import type {
  FavoriteItem,
  FavoriteKind,
  OcrFavoriteInput,
  TranslationFavoriteInput,
} from '../application/settings/ports';
import type { SettingsRuntime } from '../application/settings/runtime';

type Runtime = SettingsRuntime['favorites'];

let configuredRuntime: Runtime | null = null;

export function initializeFavoritesStore(runtime: Runtime) {
  configuredRuntime = runtime;
}

function runtime() {
  if (!configuredRuntime) throw new Error('Favorites store runtime is unavailable');
  return configuredRuntime;
}

interface FavoritesState {
  items: FavoriteItem[];
  total: number;
  revision: number;
  invalidate(): void;
  addTranslation(input: TranslationFavoriteInput): Promise<number>;
  addOcr(input: OcrFavoriteInput): Promise<number>;
  query(
    kind: FavoriteKind,
    search: string,
    tag: string,
    limit: number,
    offset: number,
  ): Promise<void>;
  updateMetadata(id: number, note: string | null, tags: string[]): Promise<void>;
  delete(id: number): Promise<void>;
}

export const useFavoritesStore = create<FavoritesState>((set) => ({
  items: [],
  total: 0,
  revision: 0,
  invalidate: () => set((state) => ({ revision: state.revision + 1 })),
  addTranslation: (input) => runtime().addTranslation(input),
  addOcr: (input) => runtime().addOcr(input),
  query: async (kind, search, tag, limit, offset) => {
    const page = await runtime().query({ kind, search, tag, limit, offset });
    set({ items: page.items, total: page.total });
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
}));
