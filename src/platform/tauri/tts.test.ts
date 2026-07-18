import { beforeEach, describe, expect, it, vi } from 'vitest';

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));

vi.mock('@tauri-apps/api/core', () => ({ invoke }));

import { systemTts } from './tts';

describe('Tauri system TTS adapter', () => {
  beforeEach(() => invoke.mockReset());

  it('lists macOS voices and invokes configured speech', async () => {
    const voices = [{ name: 'Tingting', locale: 'zh_CN' }];
    invoke.mockResolvedValueOnce(voices).mockResolvedValueOnce(undefined);

    await expect(systemTts.listVoices()).resolves.toEqual(voices);
    await systemTts.speak('你好', 'zh-CN');

    expect(invoke).toHaveBeenNthCalledWith(1, 'list_system_tts_voices');
    expect(invoke).toHaveBeenNthCalledWith(2, 'speak_text', {
      text: '你好',
      language: 'zh-CN',
    });
  });
});
