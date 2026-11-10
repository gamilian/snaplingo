import { describe, expect, it } from 'vitest';
import {
  getPinnedHoverToolbarActions,
  isClosePinnedImageShortcut,
  isCopyPinnedImageShortcut,
  isCopyPinnedTextShortcut,
  isDestroyPinnedImageShortcut,
  isOpenPinnedPreferencesShortcut,
  isQuickSavePinnedImageShortcut,
  isReplacePinnedImageShortcut,
  isSavePinnedImageShortcut,
} from './input';

describe('pinned image input mapping', () => {
  it('describes the compact hover toolbar actions', () => {
    expect(getPinnedHoverToolbarActions()).toEqual([
      {
        id: 'copy',
        label: 'Copy',
        title: 'Copy',
        ariaLabel: 'Copy pinned image',
      },
      {
        id: 'save',
        label: 'Save',
        title: 'Save',
        ariaLabel: 'Save pinned image',
      },
      {
        id: 'close',
        label: 'X',
        title: 'Close',
        ariaLabel: 'Close pinned image',
      },
    ]);
  });

  it('maps copy shortcuts without conflicting with source-text copy', () => {
    expect(
      isCopyPinnedImageShortcut({ key: 'c', metaKey: true, ctrlKey: false }),
    ).toBe(true);
    expect(
      isCopyPinnedTextShortcut(
        { key: 'c', metaKey: false, ctrlKey: true, shiftKey: true },
        'source',
      ),
    ).toBe(true);
    expect(
      isCopyPinnedTextShortcut(
        { key: 'c', metaKey: false, ctrlKey: true, shiftKey: true },
        null,
      ),
    ).toBe(false);
    expect(
      isCopyPinnedImageShortcut({
        key: 'c',
        metaKey: true,
        ctrlKey: false,
        shiftKey: true,
      }),
    ).toBe(false);
  });

  it('maps save, preferences, close, and replacement shortcuts', () => {
    expect(
      isSavePinnedImageShortcut({ key: 's', metaKey: true, ctrlKey: false }),
    ).toBe(true);
    expect(
      isQuickSavePinnedImageShortcut({
        key: 's',
        metaKey: false,
        ctrlKey: true,
        shiftKey: true,
      }),
    ).toBe(true);
    expect(
      isOpenPinnedPreferencesShortcut({
        key: 'p',
        metaKey: true,
        ctrlKey: false,
        shiftKey: true,
      }),
    ).toBe(true);
    expect(
      isClosePinnedImageShortcut({ key: 'w', metaKey: false, ctrlKey: true }),
    ).toBe(true);
    expect(
      isReplacePinnedImageShortcut({ key: 'v', metaKey: true, ctrlKey: false }),
    ).toBe(true);
  });

  it('maps only Shift+Escape to destructive close', () => {
    expect(
      isDestroyPinnedImageShortcut({ key: 'Escape', shiftKey: true }),
    ).toBe(true);
    expect(
      isDestroyPinnedImageShortcut({ key: 'Escape', shiftKey: false }),
    ).toBe(false);
    expect(
      isDestroyPinnedImageShortcut({
        key: 'Escape',
        shiftKey: true,
        metaKey: true,
      }),
    ).toBe(false);
  });
});
