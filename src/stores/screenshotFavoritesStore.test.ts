import { beforeEach, describe, expect, it, vi } from 'vitest';
import {
  initializeScreenshotFavoritesStore,
  useScreenshotFavoritesStore,
} from './screenshotFavoritesStore';

describe('screenshot favorites store', () => {
  const runtime = {
    query: vi.fn(),
    updateMetadata: vi.fn(async () => undefined),
    delete: vi.fn(async () => undefined),
    copy: vi.fn(async () => undefined),
    reveal: vi.fn(async () => undefined),
  };

  beforeEach(() => {
    vi.clearAllMocks();
    initializeScreenshotFavoritesStore(runtime);
    useScreenshotFavoritesStore.setState({ items: [], total: 0, error: null });
  });

  it('loads an authoritative page and mutates metadata locally after persistence', async () => {
    runtime.query.mockResolvedValue({
      total: 1,
      items: [
        {
          id: 1,
          createdAt: '2026-07-14T00:00:00Z',
          thumbnailDataUrl: 'data:image/png;base64,AA==',
          width: 100,
          height: 50,
          note: null,
          tags: [],
        },
      ],
    });

    await useScreenshotFavoritesStore.getState().query('work', 20, 0);
    await useScreenshotFavoritesStore
      .getState()
      .updateMetadata(1, 'keep', ['work']);

    expect(runtime.query).toHaveBeenCalledWith({
      search: 'work',
      limit: 20,
      offset: 0,
    });
    expect(runtime.updateMetadata).toHaveBeenCalledWith(1, 'keep', ['work']);
    expect(useScreenshotFavoritesStore.getState().items[0]).toMatchObject({
      note: 'keep',
      tags: ['work'],
    });
  });
});
