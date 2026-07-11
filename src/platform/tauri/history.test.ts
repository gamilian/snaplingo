import { describe, expect, it, vi } from 'vitest';

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ invoke }));

describe('Tauri history command adapter', () => {
  it('loads paginated translation history', async () => {
    const { getTranslationHistory } = await import('./history');
    invoke.mockResolvedValueOnce([]);
    await getTranslationHistory(20, 40);
    expect(invoke).toHaveBeenCalledWith('get_translation_history', { limit: 20, offset: 40 });
  });

  it('deletes a history entry by id', async () => {
    const { deleteHistory } = await import('./history');
    invoke.mockResolvedValueOnce(undefined);
    await deleteHistory(7);
    expect(invoke).toHaveBeenCalledWith('delete_history', { id: 7 });
  });
});
