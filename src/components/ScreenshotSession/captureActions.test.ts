import { describe, expect, it } from 'vitest';
import type { AnnotationCommand, LogicalRect } from './types';
import {
  type CaptureInvoke,
  type CaptureInvokeArgs,
  isCancelCapturePointer,
  isCopyCaptureDoubleClick,
  isCopyCaptureKeyboardShortcut,
  isPinCaptureShortcut,
  isSaveCaptureShortcut,
  isSelectAllCaptureShortcut,
  isToggleToolbarShortcut,
  saveCaptureSelection,
} from './captureActions';

describe('capture session actions', () => {
  it('saves the current frozen selection to the default capture path', async () => {
    const calls: Array<{ command: string; args?: unknown }> = [];
    const invoke: CaptureInvoke = async <T>(
      command: string,
      args?: CaptureInvokeArgs,
    ): Promise<T> => {
      calls.push({ command, args });
      if (command === 'default_capture_save_path') {
        return '/tmp/SnapLingo-20260617-023000.png' as T;
      }
      return undefined as T;
    };
    const rect: LogicalRect = { x: 10, y: 20, width: 30, height: 40 };
    const annotations: AnnotationCommand[] = [
      {
        type: 'rectangle',
        rect: { x: 12, y: 24, width: 10, height: 8 },
        color: [255, 0, 0, 255],
        stroke_width: 2,
      },
    ];

    await saveCaptureSelection(invoke, 'session-1', rect, annotations);

    expect(calls).toEqual([
      { command: 'default_capture_save_path', args: undefined },
      {
        command: 'output_capture',
        args: {
          sessionId: 'session-1',
          rect,
          action: {
            type: 'save',
            path: '/tmp/SnapLingo-20260617-023000.png',
          },
          annotations,
        },
      },
    ]);
  });

  it('uses Cmd/Ctrl+S for saving the current selection', () => {
    expect(
      isSaveCaptureShortcut({
        key: 's',
        metaKey: true,
        ctrlKey: false,
      }),
    ).toBe(true);
    expect(
      isSaveCaptureShortcut({
        key: 'S',
        metaKey: false,
        ctrlKey: true,
      }),
    ).toBe(true);
    expect(
      isSaveCaptureShortcut({
        key: 's',
        metaKey: false,
        ctrlKey: false,
      }),
    ).toBe(false);
    expect(
      isSaveCaptureShortcut({
        key: 'c',
        metaKey: true,
        ctrlKey: false,
      }),
    ).toBe(false);
  });

  it('uses plain F3 for pinning the current selection', () => {
    expect(
      isPinCaptureShortcut({
        key: 'F3',
        metaKey: false,
        ctrlKey: false,
        altKey: false,
        shiftKey: false,
      }),
    ).toBe(true);
    expect(
      isPinCaptureShortcut({
        key: 'F3',
        metaKey: true,
        ctrlKey: false,
        altKey: false,
        shiftKey: false,
      }),
    ).toBe(false);
    expect(
      isPinCaptureShortcut({
        key: 'F3',
        metaKey: false,
        ctrlKey: false,
        altKey: false,
        shiftKey: true,
      }),
    ).toBe(false);
    expect(
      isPinCaptureShortcut({
        key: 'p',
        metaKey: false,
        ctrlKey: false,
        altKey: false,
        shiftKey: false,
      }),
    ).toBe(false);
  });

  it('uses Cmd/Ctrl+A for selecting the full capture area', () => {
    expect(
      isSelectAllCaptureShortcut({
        key: 'a',
        metaKey: true,
        ctrlKey: false,
        altKey: false,
        shiftKey: false,
      }),
    ).toBe(true);
    expect(
      isSelectAllCaptureShortcut({
        key: 'A',
        metaKey: false,
        ctrlKey: true,
        altKey: false,
        shiftKey: false,
      }),
    ).toBe(true);
    expect(
      isSelectAllCaptureShortcut({
        key: 'a',
        metaKey: false,
        ctrlKey: false,
        altKey: false,
        shiftKey: false,
      }),
    ).toBe(false);
    expect(
      isSelectAllCaptureShortcut({
        key: 'a',
        metaKey: true,
        ctrlKey: false,
        altKey: false,
        shiftKey: true,
      }),
    ).toBe(false);
  });

  it('uses an unmodified primary-button double click for copying the current selection', () => {
    expect(
      isCopyCaptureDoubleClick({
        detail: 2,
        button: 0,
        metaKey: false,
        ctrlKey: false,
        altKey: false,
        shiftKey: false,
      }),
    ).toBe(true);
    expect(
      isCopyCaptureDoubleClick({
        detail: 1,
        button: 0,
        metaKey: false,
        ctrlKey: false,
        altKey: false,
        shiftKey: false,
      }),
    ).toBe(false);
    expect(
      isCopyCaptureDoubleClick({
        detail: 2,
        button: 2,
        metaKey: false,
        ctrlKey: false,
        altKey: false,
        shiftKey: false,
      }),
    ).toBe(false);
    expect(
      isCopyCaptureDoubleClick({
        detail: 2,
        button: 0,
        metaKey: false,
        ctrlKey: true,
        altKey: false,
        shiftKey: false,
      }),
    ).toBe(false);
  });

  it('uses Enter or Cmd/Ctrl+C for copying the current selection', () => {
    expect(
      isCopyCaptureKeyboardShortcut({
        key: 'Enter',
        metaKey: false,
        ctrlKey: false,
      }),
    ).toBe(true);
    expect(
      isCopyCaptureKeyboardShortcut({
        key: 'c',
        metaKey: true,
        ctrlKey: false,
      }),
    ).toBe(true);
    expect(
      isCopyCaptureKeyboardShortcut({
        key: 'C',
        metaKey: false,
        ctrlKey: true,
      }),
    ).toBe(true);
    expect(
      isCopyCaptureKeyboardShortcut({
        key: 'c',
        metaKey: false,
        ctrlKey: false,
      }),
    ).toBe(false);
  });

  it('uses an unmodified secondary-button press for canceling the current capture layer', () => {
    expect(
      isCancelCapturePointer({
        button: 2,
        metaKey: false,
        ctrlKey: false,
        altKey: false,
        shiftKey: false,
      }),
    ).toBe(true);
    expect(
      isCancelCapturePointer({
        button: 0,
        metaKey: false,
        ctrlKey: false,
        altKey: false,
        shiftKey: false,
      }),
    ).toBe(false);
    expect(
      isCancelCapturePointer({
        button: 2,
        metaKey: false,
        ctrlKey: false,
        altKey: true,
        shiftKey: false,
      }),
    ).toBe(false);
  });

  it('uses plain Tab for toggling the capture toolbar visibility', () => {
    expect(
      isToggleToolbarShortcut({
        key: 'Tab',
        metaKey: false,
        ctrlKey: false,
        altKey: false,
        shiftKey: false,
      }),
    ).toBe(true);
    expect(
      isToggleToolbarShortcut({
        key: 'Tab',
        metaKey: false,
        ctrlKey: false,
        altKey: false,
        shiftKey: true,
      }),
    ).toBe(false);
    expect(
      isToggleToolbarShortcut({
        key: 't',
        metaKey: false,
        ctrlKey: false,
        altKey: false,
        shiftKey: false,
      }),
    ).toBe(false);
  });
});
