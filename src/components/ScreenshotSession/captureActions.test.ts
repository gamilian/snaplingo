import { describe, expect, it } from 'vitest';
import type { AnnotationCommand, LogicalRect } from './types';
import {
  type CaptureInvoke,
  type CaptureInvokeArgs,
  copyCaptureSelection,
  getCandidateCycleDirectionFromShortcut,
  getCancelCapturePointerAction,
  getSelectionArrowActionFromShortcut,
  getSelectionHistoryStepFromShortcut,
  getUndoRedoActionFromShortcut,
  isCancelCapturePointer,
  isClearAnnotationsShortcut,
  isCopyCaptureDoubleClick,
  isCopyCaptureKeyboardShortcut,
  isConfirmHoverSelectionShortcut,
  isDeleteSelectedAnnotationShortcut,
  getCursorNudgeDeltaFromShortcut,
  isMagnifierShortcut,
  isPinCaptureShortcut,
  isPinCapturePointer,
  isPrintCaptureShortcut,
  isMoveDraftSelectionShortcut,
  isQuickSaveCaptureShortcut,
  isRestoreLastSelectionShortcut,
  isSaveCaptureShortcut,
  isSelectAllCaptureShortcut,
  isToggleToolbarShortcut,
  printCaptureSelection,
  quickSaveCaptureSelection,
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

  it('quick saves the current frozen selection to the configured capture path', async () => {
    const calls: Array<{ command: string; args?: unknown }> = [];
    const invoke: CaptureInvoke = async <T>(
      command: string,
      args?: CaptureInvokeArgs,
    ): Promise<T> => {
      calls.push({ command, args });
      if (command === 'quick_capture_save_path') {
        return '/Users/alice/Pictures/SnapLingo/SnapLingo-20260617-023000.png' as T;
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

    await quickSaveCaptureSelection(
      invoke,
      'session-1',
      rect,
      annotations,
      '~/Pictures/SnapLingo',
    );

    expect(calls).toEqual([
      {
        command: 'quick_capture_save_path',
        args: { directory: '~/Pictures/SnapLingo' },
      },
      {
        command: 'output_capture',
        args: {
          sessionId: 'session-1',
          rect,
          action: {
            type: 'save',
            path: '/Users/alice/Pictures/SnapLingo/SnapLingo-20260617-023000.png',
          },
          annotations,
        },
      },
    ]);
  });

  it('uses Cmd/Ctrl+Shift+S for quick saving the current selection', () => {
    expect(
      isQuickSaveCaptureShortcut({
        key: 's',
        metaKey: true,
        ctrlKey: false,
        altKey: false,
        shiftKey: true,
      }),
    ).toBe(true);
    expect(
      isQuickSaveCaptureShortcut({
        key: 'S',
        metaKey: false,
        ctrlKey: true,
        altKey: false,
        shiftKey: true,
      }),
    ).toBe(true);
    expect(
      isQuickSaveCaptureShortcut({
        key: 's',
        metaKey: true,
        ctrlKey: false,
        altKey: false,
        shiftKey: false,
      }),
    ).toBe(false);
    expect(
      isQuickSaveCaptureShortcut({
        key: 's',
        metaKey: true,
        ctrlKey: false,
        altKey: true,
        shiftKey: true,
      }),
    ).toBe(false);
  });

  it('uses plain R for restoring the last selection', () => {
    expect(
      isRestoreLastSelectionShortcut({
        key: 'r',
        metaKey: false,
        ctrlKey: false,
        altKey: false,
        shiftKey: false,
      }),
    ).toBe(true);
    expect(
      isRestoreLastSelectionShortcut({
        key: 'R',
        metaKey: false,
        ctrlKey: false,
        altKey: false,
        shiftKey: false,
      }),
    ).toBe(true);
    expect(
      isRestoreLastSelectionShortcut({
        key: 'r',
        metaKey: true,
        ctrlKey: false,
        altKey: false,
        shiftKey: false,
      }),
    ).toBe(false);
    expect(
      isRestoreLastSelectionShortcut({
        key: 'r',
        metaKey: false,
        ctrlKey: false,
        altKey: false,
        shiftKey: true,
      }),
    ).toBe(false);
  });

  it('uses comma and period for cycling capture selection history', () => {
    expect(
      getSelectionHistoryStepFromShortcut({
        key: ',',
        metaKey: false,
        ctrlKey: false,
        altKey: false,
        shiftKey: false,
      }),
    ).toBe('previous');
    expect(
      getSelectionHistoryStepFromShortcut({
        key: '.',
        metaKey: false,
        ctrlKey: false,
        altKey: false,
        shiftKey: false,
      }),
    ).toBe('next');
    expect(
      getSelectionHistoryStepFromShortcut({
        key: ',',
        metaKey: true,
        ctrlKey: false,
        altKey: false,
        shiftKey: false,
      }),
    ).toBeNull();
    expect(
      getSelectionHistoryStepFromShortcut({
        key: '.',
        metaKey: false,
        ctrlKey: false,
        altKey: false,
        shiftKey: true,
      }),
    ).toBeNull();
  });

  it('uses Cmd/Ctrl+Shift+Z for clearing all annotations', () => {
    expect(
      isClearAnnotationsShortcut({
        key: 'z',
        metaKey: true,
        ctrlKey: false,
        altKey: false,
        shiftKey: true,
      }),
    ).toBe(true);
    expect(
      isClearAnnotationsShortcut({
        key: 'Z',
        metaKey: false,
        ctrlKey: true,
        altKey: false,
        shiftKey: true,
      }),
    ).toBe(true);
    expect(
      isClearAnnotationsShortcut({
        key: 'z',
        metaKey: true,
        ctrlKey: false,
        altKey: false,
        shiftKey: false,
      }),
    ).toBe(false);
    expect(
      isClearAnnotationsShortcut({
        key: 'z',
        metaKey: true,
        ctrlKey: false,
        altKey: true,
        shiftKey: true,
      }),
    ).toBe(false);
  });

  it('maps Snipaste undo and redo shortcuts without stealing clear annotations', () => {
    expect(
      getUndoRedoActionFromShortcut({
        key: 'z',
        metaKey: true,
        ctrlKey: false,
        altKey: false,
        shiftKey: false,
      }),
    ).toBe('undo');
    expect(
      getUndoRedoActionFromShortcut({
        key: 'Z',
        metaKey: false,
        ctrlKey: true,
        altKey: false,
        shiftKey: false,
      }),
    ).toBe('undo');
    expect(
      getUndoRedoActionFromShortcut({
        key: 'y',
        metaKey: true,
        ctrlKey: false,
        altKey: false,
        shiftKey: false,
      }),
    ).toBe('redo');
    expect(
      getUndoRedoActionFromShortcut({
        key: 'z',
        metaKey: true,
        ctrlKey: false,
        altKey: false,
        shiftKey: true,
      }),
    ).toBeNull();
    expect(
      getUndoRedoActionFromShortcut({
        key: 'y',
        metaKey: true,
        ctrlKey: false,
        altKey: true,
        shiftKey: false,
      }),
    ).toBeNull();
  });

  it('uses plain Backspace or Delete for deleting the selected annotation', () => {
    expect(
      isDeleteSelectedAnnotationShortcut({
        key: 'Backspace',
        metaKey: false,
        ctrlKey: false,
        altKey: false,
        shiftKey: false,
      }),
    ).toBe(true);
    expect(
      isDeleteSelectedAnnotationShortcut({
        key: 'Delete',
        metaKey: false,
        ctrlKey: false,
        altKey: false,
        shiftKey: false,
      }),
    ).toBe(true);
    expect(
      isDeleteSelectedAnnotationShortcut({
        key: 'Delete',
        metaKey: true,
        ctrlKey: false,
        altKey: false,
        shiftKey: false,
      }),
    ).toBe(false);
  });

  it('maps Snipaste selection arrow shortcuts to move, expand, or shrink', () => {
    expect(
      getSelectionArrowActionFromShortcut({
        key: 'ArrowUp',
        metaKey: false,
        ctrlKey: false,
        altKey: false,
        shiftKey: false,
      }),
    ).toEqual({ mode: 'move', direction: 'ArrowUp' });
    expect(
      getSelectionArrowActionFromShortcut({
        key: 'ArrowRight',
        metaKey: false,
        ctrlKey: true,
        altKey: false,
        shiftKey: false,
      }),
    ).toEqual({ mode: 'expand', direction: 'ArrowRight' });
    expect(
      getSelectionArrowActionFromShortcut({
        key: 'ArrowRight',
        metaKey: true,
        ctrlKey: false,
        altKey: false,
        shiftKey: false,
      }),
    ).toEqual({ mode: 'expand', direction: 'ArrowRight' });
    expect(
      getSelectionArrowActionFromShortcut({
        key: 'ArrowLeft',
        metaKey: false,
        ctrlKey: false,
        altKey: false,
        shiftKey: true,
      }),
    ).toEqual({ mode: 'shrink', direction: 'ArrowLeft' });
    expect(
      getSelectionArrowActionFromShortcut({
        key: 'ArrowDown',
        metaKey: false,
        ctrlKey: true,
        altKey: false,
        shiftKey: true,
      }),
    ).toBeNull();
    expect(
      getSelectionArrowActionFromShortcut({
        key: 'ArrowDown',
        metaKey: false,
        ctrlKey: false,
        altKey: true,
        shiftKey: false,
      }),
    ).toBeNull();
    expect(
      getSelectionArrowActionFromShortcut({
        key: 'w',
        metaKey: false,
        ctrlKey: false,
        altKey: false,
        shiftKey: false,
      }),
    ).toBeNull();
  });

  it('copies the current frozen selection to the clipboard', async () => {
    const calls: Array<{ command: string; args?: unknown }> = [];
    const invoke: CaptureInvoke = async <T>(
      command: string,
      args?: CaptureInvokeArgs,
    ): Promise<T> => {
      calls.push({ command, args });
      return undefined as T;
    };
    const rect: LogicalRect = { x: 12, y: 24, width: 120, height: 80 };
    const annotations: AnnotationCommand[] = [
      {
        type: 'arrow',
        start: { x: 4, y: 8 },
        end: { x: 48, y: 32 },
        color: [0, 128, 255, 255],
        stroke_width: 3,
      },
    ];

    await copyCaptureSelection(invoke, 'session-2', rect, annotations);

    expect(calls).toEqual([
      {
        command: 'output_capture',
        args: {
          sessionId: 'session-2',
          rect,
          annotations,
          action: { type: 'copy' },
        },
      },
    ]);
  });

  it('prints the rendered current frozen selection', async () => {
    const calls: Array<{ command: string; args?: unknown }> = [];
    const printedImages: string[] = [];
    const invoke: CaptureInvoke = async <T>(
      command: string,
      args?: CaptureInvokeArgs,
    ): Promise<T> => {
      calls.push({ command, args });
      return 'rendered-png-base64' as T;
    };
    const rect: LogicalRect = { x: 12, y: 24, width: 120, height: 80 };
    const annotations: AnnotationCommand[] = [
      {
        type: 'arrow',
        start: { x: 4, y: 8 },
        end: { x: 48, y: 32 },
        color: [0, 128, 255, 255],
        stroke_width: 3,
      },
    ];

    await printCaptureSelection(
      invoke,
      'session-2',
      rect,
      annotations,
      async (imageBase64) => {
        printedImages.push(imageBase64);
      },
    );

    expect(calls).toEqual([
      {
        command: 'render_capture_output',
        args: {
          sessionId: 'session-2',
          rect,
          annotations,
        },
      },
    ]);
    expect(printedImages).toEqual(['rendered-png-base64']);
  });

  it('uses Cmd/Ctrl+P for printing the current selection', () => {
    expect(
      isPrintCaptureShortcut({
        key: 'p',
        metaKey: true,
        ctrlKey: false,
        altKey: false,
        shiftKey: false,
      }),
    ).toBe(true);
    expect(
      isPrintCaptureShortcut({
        key: 'P',
        metaKey: false,
        ctrlKey: true,
        altKey: false,
        shiftKey: false,
      }),
    ).toBe(true);
    expect(
      isPrintCaptureShortcut({
        key: 'p',
        metaKey: true,
        ctrlKey: false,
        altKey: false,
        shiftKey: true,
      }),
    ).toBe(false);
    expect(
      isPrintCaptureShortcut({
        key: 'p',
        metaKey: false,
        ctrlKey: false,
        altKey: false,
        shiftKey: false,
      }),
    ).toBe(false);
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
    expect(
      isSaveCaptureShortcut({
        key: 's',
        metaKey: true,
        ctrlKey: false,
        altKey: false,
        shiftKey: true,
      }),
    ).toBe(false);
    expect(
      isSaveCaptureShortcut({
        key: 's',
        metaKey: false,
        ctrlKey: true,
        altKey: true,
        shiftKey: false,
      }),
    ).toBe(false);
  });

  it('uses Cmd/Ctrl+T for pinning the current selection', () => {
    expect(
      isPinCaptureShortcut({
        key: 't',
        metaKey: true,
        ctrlKey: false,
        altKey: false,
        shiftKey: false,
      }),
    ).toBe(true);
    expect(
      isPinCaptureShortcut({
        key: 'T',
        metaKey: false,
        ctrlKey: true,
        altKey: false,
        shiftKey: false,
      }),
    ).toBe(true);
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
        key: 'F3',
        metaKey: false,
        ctrlKey: false,
        altKey: false,
        shiftKey: false,
      }),
    ).toBe(false);
  });

  it('maps plain WASD keys to one-pixel cursor movement', () => {
    expect(
      getCursorNudgeDeltaFromShortcut({
        key: 'w',
        metaKey: false,
        ctrlKey: false,
        altKey: false,
        shiftKey: false,
      }),
    ).toEqual({ x: 0, y: -1 });
    expect(
      getCursorNudgeDeltaFromShortcut({
        key: 'A',
        metaKey: false,
        ctrlKey: false,
        altKey: false,
        shiftKey: false,
      }),
    ).toEqual({ x: -1, y: 0 });
    expect(
      getCursorNudgeDeltaFromShortcut({
        key: 's',
        metaKey: false,
        ctrlKey: false,
        altKey: false,
        shiftKey: false,
      }),
    ).toEqual({ x: 0, y: 1 });
    expect(
      getCursorNudgeDeltaFromShortcut({
        key: 'd',
        metaKey: false,
        ctrlKey: false,
        altKey: false,
        shiftKey: false,
      }),
    ).toEqual({ x: 1, y: 0 });
    expect(
      getCursorNudgeDeltaFromShortcut({
        key: 'w',
        metaKey: true,
        ctrlKey: false,
        altKey: false,
        shiftKey: false,
      }),
    ).toBeNull();
  });

  it('uses plain Alt for showing the magnifier', () => {
    expect(
      isMagnifierShortcut({
        key: 'Alt',
        metaKey: false,
        ctrlKey: false,
        altKey: true,
        shiftKey: false,
      }),
    ).toBe(true);
    expect(
      isMagnifierShortcut({
        key: 'Alt',
        metaKey: false,
        ctrlKey: true,
        altKey: true,
        shiftKey: false,
      }),
    ).toBe(false);
    expect(
      isMagnifierShortcut({
        key: 'Shift',
        metaKey: false,
        ctrlKey: false,
        altKey: false,
        shiftKey: true,
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

  it('uses an unmodified middle-button press for pinning the current selection', () => {
    expect(
      isPinCapturePointer({
        button: 1,
        metaKey: false,
        ctrlKey: false,
        altKey: false,
        shiftKey: false,
      }),
    ).toBe(true);
    expect(
      isPinCapturePointer({
        button: 0,
        metaKey: false,
        ctrlKey: false,
        altKey: false,
        shiftKey: false,
      }),
    ).toBe(false);
    expect(
      isPinCapturePointer({
        button: 1,
        metaKey: true,
        ctrlKey: false,
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
        key: ' ',
        metaKey: false,
        ctrlKey: false,
        altKey: false,
        shiftKey: false,
      }),
    ).toBe(false);
    expect(
      isCopyCaptureKeyboardShortcut({
        key: 'c',
        metaKey: false,
        ctrlKey: false,
      }),
    ).toBe(false);
    expect(
      isCopyCaptureKeyboardShortcut({
        key: 'c',
        metaKey: false,
        ctrlKey: true,
        altKey: false,
        shiftKey: true,
      }),
    ).toBe(false);
    expect(
      isCopyCaptureKeyboardShortcut({
        key: 'c',
        metaKey: false,
        ctrlKey: true,
        altKey: true,
        shiftKey: false,
      }),
    ).toBe(false);
    expect(
      isCopyCaptureKeyboardShortcut({
        key: ' ',
        metaKey: false,
        ctrlKey: false,
        altKey: false,
        shiftKey: true,
      }),
    ).toBe(false);
    expect(
      isCopyCaptureKeyboardShortcut({
        key: 'Enter',
        metaKey: true,
        ctrlKey: false,
        altKey: false,
        shiftKey: false,
      }),
    ).toBe(false);
  });

  it('uses unmodified Enter for confirming a hovered capture candidate', () => {
    expect(
      isConfirmHoverSelectionShortcut({
        key: 'Enter',
        metaKey: false,
        ctrlKey: false,
        altKey: false,
        shiftKey: false,
      }),
    ).toBe(true);
    expect(
      isConfirmHoverSelectionShortcut({
        key: 'Enter',
        metaKey: true,
        ctrlKey: false,
        altKey: false,
        shiftKey: false,
      }),
    ).toBe(false);
    expect(
      isConfirmHoverSelectionShortcut({
        key: ' ',
        metaKey: false,
        ctrlKey: false,
        altKey: false,
        shiftKey: false,
      }),
    ).toBe(false);
  });

  it('maps Tab shortcuts to capture candidate cycle direction', () => {
    expect(
      getCandidateCycleDirectionFromShortcut({
        key: 'Tab',
        metaKey: false,
        ctrlKey: false,
        altKey: false,
        shiftKey: false,
      }),
    ).toBe(1);
    expect(
      getCandidateCycleDirectionFromShortcut({
        key: 'Tab',
        metaKey: false,
        ctrlKey: false,
        altKey: false,
        shiftKey: true,
      }),
    ).toBe(-1);
    expect(
      getCandidateCycleDirectionFromShortcut({
        key: 'Tab',
        metaKey: true,
        ctrlKey: false,
        altKey: false,
        shiftKey: false,
      }),
    ).toBeNull();
    expect(
      getCandidateCycleDirectionFromShortcut({
        key: 'Enter',
        metaKey: false,
        ctrlKey: false,
        altKey: false,
        shiftKey: false,
      }),
    ).toBeNull();
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

  it('resets an existing preview selection before canceling the capture session', () => {
    expect(
      getCancelCapturePointerAction({
        status: 'preview',
        hasSelection: true,
        hasDismissibleLayer: false,
      }),
    ).toBe('reset-selection');
    expect(
      getCancelCapturePointerAction({
        status: 'selecting',
        hasSelection: false,
        hasDismissibleLayer: false,
      }),
    ).toBe('cancel-session');
  });

  it('dismisses active edit layers before resetting the preview selection', () => {
    expect(
      getCancelCapturePointerAction({
        status: 'preview',
        hasSelection: true,
        hasDismissibleLayer: true,
      }),
    ).toBe('dismiss-layer');
  });

  it('uses plain Space for toggling the capture toolbar visibility', () => {
    expect(
      isToggleToolbarShortcut({
        key: ' ',
        metaKey: false,
        ctrlKey: false,
        altKey: false,
        shiftKey: false,
      }),
    ).toBe(true);
    expect(
      isToggleToolbarShortcut({
        key: ' ',
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

  it('uses Space for moving an in-progress selection without system modifiers', () => {
    expect(
      isMoveDraftSelectionShortcut({
        key: ' ',
        metaKey: false,
        ctrlKey: false,
        altKey: false,
        shiftKey: false,
      }),
    ).toBe(true);
    expect(
      isMoveDraftSelectionShortcut({
        key: ' ',
        metaKey: false,
        ctrlKey: false,
        altKey: false,
        shiftKey: true,
      }),
    ).toBe(true);
    expect(
      isMoveDraftSelectionShortcut({
        key: ' ',
        metaKey: true,
        ctrlKey: false,
        altKey: false,
        shiftKey: false,
      }),
    ).toBe(false);
    expect(
      isMoveDraftSelectionShortcut({
        key: ' ',
        metaKey: false,
        ctrlKey: false,
        altKey: true,
        shiftKey: false,
      }),
    ).toBe(false);
  });
});
