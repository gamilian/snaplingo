import { create } from 'zustand';
import type {
  FavoriteItem,
  FavoriteKind,
  OcrFavoriteInput,
  TranslationFavoriteInput,
} from '../application/settings/ports';
import type { SettingsRuntime } from '../application/settings/runtime';
import {
  ocrFavoriteKey,
  translationFavoriteKey,
} from '../application/favorites/identity';

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
  keys: Set<string>;
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
  hydrateKeys(kind: FavoriteKind): Promise<void>;
}

function itemKey(item: FavoriteItem) {
  if (item.content.contentKind === 'translation') {
    const snapshot = item.content.snapshot;
    return translationFavoriteKey({
      sourceText: snapshot.sourceText,
      sourceLang: snapshot.sourceLang,
      targetLang: snapshot.targetLang,
      providerId: snapshot.result.provider_id,
      translatedText: snapshot.result.translated_text,
    });
  }
  const snapshot = item.content.snapshot;
  return ocrFavoriteKey({
    recognizedText: snapshot.recognizedText,
    language: snapshot.language,
    providerUsed: snapshot.providerUsed,
  });
}

export const useFavoritesStore = create<FavoritesState>((set) => ({
  items: [],
  total: 0,
  revision: 0,
  keys: new Set(),
  invalidate: () => set((state) => ({ revision: state.revision + 1 })),
  addTranslation: async (input) => {
    const id = await runtime().addTranslation(input);
    set((state) => ({
      keys: new Set(state.keys).add(translationFavoriteKey(input)),
      revision: state.revision + 1,
    }));
    return id;
  },
  addOcr: async (input) => {
    const id = await runtime().addOcr(input);
    set((state) => ({
      keys: new Set(state.keys).add(
        ocrFavoriteKey({
          recognizedText: input.recognizedText,
          language: input.language ?? null,
          providerUsed: input.providerUsed,
        }),
      ),
      revision: state.revision + 1,
    }));
    return id;
  },
  query: async (kind, search, tag, limit, offset) => {
    const page = await runtime().query({ kind, search, tag, limit, offset });
    set((state) => ({
      items: page.items,
      total: page.total,
      keys: new Set([...state.keys, ...page.items.map(itemKey)]),
    }));
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
    set((state) => {
      const deleted = state.items.find((item) => item.id === id);
      const keys = new Set(state.keys);
      if (deleted) keys.delete(itemKey(deleted));
      return {
        items: state.items.filter((item) => item.id !== id),
        total: Math.max(0, state.total - 1),
        keys,
        revision: state.revision + 1,
      };
    });
  },
  hydrateKeys: async (kind) => {
    const page = await runtime().query({ kind, limit: 1000, offset: 0 });
    set((state) => {
      const prefix = `[\"${kind}\"`;
      const keys = new Set(
        [...state.keys].filter((key) => !key.startsWith(prefix)),
      );
      page.items.forEach((item) => keys.add(itemKey(item)));
      return { keys };
    });
  },
}));
