import { beforeEach, describe, expect, it, vi } from 'vitest';
import { speakResultWindowText } from './speech';

describe('result window speech', () => {
  beforeEach(() => {
    vi.unstubAllGlobals();
  });

  it('ignores empty text', () => {
    const cancel = vi.fn();
    const speak = vi.fn();
    const SpeechSynthesisUtteranceMock = vi.fn(function (
      this: { text: string; lang: string },
      text: string,
    ) {
      this.text = text;
      this.lang = '';
    });

    vi.stubGlobal('speechSynthesis', { cancel, speak });
    vi.stubGlobal('SpeechSynthesisUtterance', SpeechSynthesisUtteranceMock);

    expect(speakResultWindowText('   ')).toBe(false);
    expect(cancel).not.toHaveBeenCalled();
    expect(speak).not.toHaveBeenCalled();
  });

  it('cancels the previous utterance and speaks trimmed text', () => {
    const cancel = vi.fn();
    const speak = vi.fn();
    const SpeechSynthesisUtteranceMock = vi.fn(function (
      this: { text: string; lang: string },
      text: string,
    ) {
      this.text = text;
      this.lang = '';
    });

    vi.stubGlobal('speechSynthesis', { cancel, speak });
    vi.stubGlobal('SpeechSynthesisUtterance', SpeechSynthesisUtteranceMock);

    expect(speakResultWindowText('\n  hello world  \n', 'en')).toBe(true);
    expect(cancel).toHaveBeenCalledTimes(1);
    expect(SpeechSynthesisUtteranceMock).toHaveBeenCalledWith('hello world');
    expect(speak).toHaveBeenCalledTimes(1);
    expect(speak.mock.calls[0]?.[0]).toMatchObject({
      text: 'hello world',
      lang: 'en-US',
    });
  });
});
