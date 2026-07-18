import { describe, expect, it, vi } from 'vitest';
import { speakResultWindowText } from './speech';

describe('result window speech', () => {
  it('ignores empty text', async () => {
    const speech = { speak: vi.fn(async () => undefined) };
    await expect(speakResultWindowText(speech, '   ')).resolves.toBe(false);
    expect(speech.speak).not.toHaveBeenCalled();
  });

  it('speaks trimmed text with a normalized language', async () => {
    const speech = { speak: vi.fn(async () => undefined) };
    await expect(
      speakResultWindowText(speech, '\n  hello world  \n', 'en'),
    ).resolves.toBe(true);
    expect(speech.speak).toHaveBeenCalledWith('hello world', 'en-US');
  });
});
