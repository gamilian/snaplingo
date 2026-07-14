import { describe, expect, it } from 'vitest';
import { mergeTimestampedPage } from './libraryPagination';

describe('library pagination', () => {
  it('merges independently queried prefixes before applying global pagination', () => {
    const translation = [
      { id: 't1', timestamp: 90 },
      { id: 't2', timestamp: 60 },
      { id: 't3', timestamp: 30 },
    ];
    const ocr = [
      { id: 'o1', timestamp: 100 },
      { id: 'o2', timestamp: 80 },
      { id: 'o3', timestamp: 40 },
    ];

    expect(mergeTimestampedPage([translation, ocr], 2, 3)).toEqual([
      { id: 'o2', timestamp: 80 },
      { id: 't2', timestamp: 60 },
      { id: 'o3', timestamp: 40 },
    ]);
  });
});
