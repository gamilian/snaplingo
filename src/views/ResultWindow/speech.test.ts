import { describe, expect, it, vi } from 'vitest';
import { speakResultWindowText } from './speech';

describe('result window speech', () => {
  it('ignores empty text', () => {
    const speak = vi.fn(async () => undefined);
    expect(speakResultWindowText(speak, '   ')).toBe(false);
    expect(speak).not.toHaveBeenCalled();
  });

  it('speaks trimmed text with a normalized language', () => {
    const speak = vi.fn(async () => undefined);
    expect(speakResultWindowText(speak, '\n  hello world  \n', 'en')).toBe(true);
    expect(speak).toHaveBeenCalledWith('hello world', 'en-US');
  });
});
