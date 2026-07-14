import { beforeEach, describe, expect, it, vi } from 'vitest';

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));

vi.mock('@tauri-apps/api/core', () => ({ invoke }));

import { queryFavoriteIndex, queryHistoryIndex } from './libraryIndex';

describe('library index adapter', () => {
  beforeEach(() => invoke.mockReset());

  it('uses the typed history and favorite index commands', async () => {
    invoke.mockResolvedValue({ items: [], total: 0 });
    const query = { search: 'snap', limit: 20, offset: 40 };

    await queryHistoryIndex(query);
    await queryFavoriteIndex(query);

    expect(invoke).toHaveBeenNthCalledWith(1, 'query_library_history_index', {
      query,
    });
    expect(invoke).toHaveBeenNthCalledWith(2, 'query_library_favorite_index', {
      query,
    });
  });
});
