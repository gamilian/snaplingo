import { describe, expect, it } from 'vitest';
import { referenceUrl } from './App';

describe('browser visual reference entry', () => {
  it.each([
    ['', 'settings'],
    ['?page=services&route=tts&appearance=dark', 'settings'],
    ['?surface=result&mode=ocr&scenario=image', 'result'],
    ['?window=capture-result&theme=paper', 'result'],
    ['?surface=permission&scenario=attempted', 'permission'],
  ])('keeps fixture parameters for %s', (search, reference) => {
    expect(referenceUrl(search)).toBe(
      `/reference/web/fixtures/${reference}_reference.html${search}`,
    );
  });
});
