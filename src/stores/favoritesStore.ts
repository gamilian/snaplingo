import { create } from 'zustand';
import type {
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
  if (!configuredRuntime)
    throw new Error('Favorites store runtime is unavailable');
  return configuredRuntime;
}

interface FavoritesState {
  revision: number;
  invalidate(): void;
  addTranslation(input: TranslationFavoriteInput): Promise<number>;
  addOcr(input: OcrFavoriteInput): Promise<number>;
}

export const useFavoritesStore = create<FavoritesState>((set) => ({
  revision: 0,
  invalidate: () => set((state) => ({ revision: state.revision + 1 })),
  addTranslation: async (input) => {
    const id = await runtime().addTranslation(input);
    set((state) => ({
      revision: state.revision + 1,
    }));
    return id;
  },
  addOcr: async (input) => {
    const id = await runtime().addOcr(input);
    set((state) => ({
      revision: state.revision + 1,
    }));
    return id;
  },
}));
