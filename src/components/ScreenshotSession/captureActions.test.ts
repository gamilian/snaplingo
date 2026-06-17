import { describe, expect, it } from 'vitest';
import type { AnnotationCommand, CaptureSessionView, LogicalRect } from './types';
import {
  type CaptureInvoke,
  type CaptureInvokeArgs,
  canToggleCapturedCursor,
  copyCaptureSelection,
  getCaptureKeyboardToolbarAction,
  getCaptureSelectionFlowForMode,
  getCandidateCycleDirectionFromShortcut,
  getCancelCapturePointerAction,
  getSelectionArrowActionFromShortcut,
  getSelectionHistoryStepFromShortcut,
  getUndoRedoActionFromShortcut,
  isCancelCapturePointer,
  isClearAnnotationsShortcut,
  isCopyCaptureDoubleClick,
  isCopyCaptureKeyboardShortcut,
  isDeleteSelectedAnnotationShortcut,
  isFinishAnnotationGestureDoubleClick,
  isUndoAnnotationGesturePointShortcut,
  getCursorNudgeDeltaFromShortcut,
  getSaveCapturePointerAction,
  getHoverSelectionCompletionActionFromShortcut,
  isMagnifierShortcut,
  isPinCaptureShortcut,
  isPinCapturePointer,
  isPrintCaptureShortcut,
  isRefreshCaptureShortcut,
  isMoveDraftSelectionShortcut,
  isQuickSaveCaptureShortcut,
  isRestoreLastSelectionShortcut,
  isSaveCaptureShortcut,
  isSelectAllCaptureShortcut,
  isToggleCapturedCursorShortcut,
  printCaptureSelection,
  quickSaveCaptureSelection,
  refreshCaptureSession,
  saveCaptureSelection,
  shouldRestoreLastSelectionFromShortcut,
  shouldCancelCaptureOnBlur,
  shouldRecordSuccessfulCaptureSelection,
  shouldCopyHoverSelectionFromShortcut,
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
        filled: false,
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
        filled: false,
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

  it('refreshes the frozen capture session before removing the previous one', async () => {
    const calls: Array<{ command: string; args?: unknown }> = [];
    const nextSession: CaptureSessionView = {
      id: 'session-next',
      monitors: [],
      candidates: [],
    };
    const invoke: CaptureInvoke = async <T>(
      command: string,
      args?: CaptureInvokeArgs,
    ): Promise<T> => {
      calls.push({ command, args });
      if (command === 'create_capture_session') {
        return nextSession as T;
      }
      return undefined as T;
    };

    const session = await refreshCaptureSession(invoke, 'session-prev');

    expect(session).toBe(nextSession);
    expect(calls).toEqual([
      { command: 'create_capture_session', args: undefined },
      {
        command: 'cancel_capture_session',
        args: { sessionId: 'session-prev' },
      },
    ]);
  });

  it('records only Snipaste-defined successful screenshot actions in selection history', () => {
    expect(shouldRecordSuccessfulCaptureSelection('copy')).toBe(true);
    expect(shouldRecordSuccessfulCaptureSelection('save')).toBe(true);
    expect(shouldRecordSuccessfulCaptureSelection('quick-save')).toBe(true);
    expect(shouldRecordSuccessfulCaptureSelection('pin')).toBe(true);

    expect(shouldRecordSuccessfulCaptureSelection('ocr')).toBe(false);
    expect(shouldRecordSuccessfulCaptureSelection('print')).toBe(false);
    expect(shouldRecordSuccessfulCaptureSelection('cancel')).toBe(false);
  });

  it('chooses the completion flow from the capture mode', () => {
    expect(getCaptureSelectionFlowForMode('screenshot')).toBe('preview');
    expect(getCaptureSelectionFlowForMode('screenshot-ocr')).toBe('ocr');
    expect(getCaptureSelectionFlowForMode('screenshot-translate')).toBe(
      'ocr-translate',
    );
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

  it('restores the last selection from selecting and preview states only when not editing', () => {
    const plainRestore = {
      key: 'r',
      metaKey: false,
      ctrlKey: false,
      altKey: false,
      shiftKey: false,
    };

    expect(
      shouldRestoreLastSelectionFromShortcut(plainRestore, {
        status: 'selecting',
      }),
    ).toBe(true);
    expect(
      shouldRestoreLastSelectionFromShortcut(plainRestore, {
        status: 'preview',
      }),
    ).toBe(true);
    expect(
      shouldRestoreLastSelectionFromShortcut(plainRestore, {
        status: 'preview',
        editing: true,
      }),
    ).toBe(false);
    expect(
      shouldRestoreLastSelectionFromShortcut(plainRestore, {
        status: 'idle',
      }),
    ).toBe(false);
    expect(
      shouldRestoreLastSelectionFromShortcut(
        { ...plainRestore, metaKey: true },
        { status: 'preview' },
      ),
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
        key: 'Backspace',
        metaKey: true,
        ctrlKey: false,
        altKey: false,
        shiftKey: true,
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

  it('uses plain Backspace or Delete for undoing an active annotation gesture point', () => {
    expect(
      isUndoAnnotationGesturePointShortcut({
        key: 'Backspace',
        metaKey: false,
        ctrlKey: false,
        altKey: false,
        shiftKey: false,
      }),
    ).toBe(true);
    expect(
      isUndoAnnotationGesturePointShortcut({
        key: 'Delete',
        metaKey: false,
        ctrlKey: false,
        altKey: false,
        shiftKey: false,
      }),
    ).toBe(true);
    expect(
      isUndoAnnotationGesturePointShortcut({
        key: 'Backspace',
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

  it('does not move the capture region with arrow keys while editing annotations', () => {
    expect(
      getSelectionArrowActionFromShortcut(
        {
          key: 'ArrowUp',
          metaKey: false,
          ctrlKey: false,
          altKey: false,
          shiftKey: false,
        },
        { editing: true },
      ),
    ).toBeNull();
    expect(
      getSelectionArrowActionFromShortcut(
        {
          key: 'ArrowRight',
          metaKey: true,
          ctrlKey: false,
          altKey: false,
          shiftKey: false,
        },
        { editing: true },
      ),
    ).toBeNull();
    expect(
      getSelectionArrowActionFromShortcut(
        {
          key: 'ArrowLeft',
          metaKey: false,
          ctrlKey: false,
          altKey: false,
          shiftKey: true,
        },
        { editing: true },
      ),
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

  it('requests cursor composition when copying with captured cursor enabled', async () => {
    const calls: Array<{ command: string; args?: unknown }> = [];
    const invoke: CaptureInvoke = async <T>(
      command: string,
      args?: CaptureInvokeArgs,
    ): Promise<T> => {
      calls.push({ command, args });
      return undefined as T;
    };
    const rect: LogicalRect = { x: 12, y: 24, width: 120, height: 80 };

    await copyCaptureSelection(invoke, 'session-2', rect, [], true);

    expect(calls).toEqual([
      {
        command: 'output_capture',
        args: {
          sessionId: 'session-2',
          rect,
          annotations: [],
          includeCursor: true,
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

  it('requests cursor composition when printing with captured cursor enabled', async () => {
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

    await printCaptureSelection(
      invoke,
      'session-2',
      rect,
      [],
      async (imageBase64) => {
        printedImages.push(imageBase64);
      },
      true,
    );

    expect(calls).toEqual([
      {
        command: 'render_capture_output',
        args: {
          sessionId: 'session-2',
          rect,
          annotations: [],
          includeCursor: true,
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

  it('uses Shift plus toolbar save click for quick saving', () => {
    expect(
      getSaveCapturePointerAction({
        button: 0,
        metaKey: false,
        ctrlKey: false,
        altKey: false,
        shiftKey: false,
      }),
    ).toBe('save');
    expect(
      getSaveCapturePointerAction({
        button: 0,
        metaKey: false,
        ctrlKey: false,
        altKey: false,
        shiftKey: true,
      }),
    ).toBe('quick-save');
    expect(
      getSaveCapturePointerAction({
        button: 0,
        metaKey: true,
        ctrlKey: false,
        altKey: false,
        shiftKey: true,
      }),
    ).toBe('save');
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

  it('uses plain F5 for refreshing the frozen screenshot', () => {
    expect(
      isRefreshCaptureShortcut({
        key: 'F5',
        metaKey: false,
        ctrlKey: false,
        altKey: false,
        shiftKey: false,
      }),
    ).toBe(true);
    expect(
      isRefreshCaptureShortcut({
        key: 'F5',
        metaKey: false,
        ctrlKey: true,
        altKey: false,
        shiftKey: false,
      }),
    ).toBe(false);
    expect(
      isRefreshCaptureShortcut({
        key: 'r',
        metaKey: false,
        ctrlKey: false,
        altKey: false,
        shiftKey: false,
      }),
    ).toBe(false);
  });

  it('uses Snipaste cursor capture toggle shortcuts only when cursor data exists', () => {
    expect(
      isToggleCapturedCursorShortcut({
        key: '`',
        metaKey: false,
        ctrlKey: false,
        altKey: false,
        shiftKey: false,
      }),
    ).toBe(true);
    expect(
      isToggleCapturedCursorShortcut({
        key: '!',
        metaKey: false,
        ctrlKey: false,
        altKey: false,
        shiftKey: true,
      }),
    ).toBe(true);
    expect(
      isToggleCapturedCursorShortcut({
        key: '`',
        metaKey: true,
        ctrlKey: false,
        altKey: false,
        shiftKey: false,
      }),
    ).toBe(false);
    expect(
      canToggleCapturedCursor({
        id: 'session-1',
        monitors: [],
        candidates: [],
        captured_cursor: {
          logical_position: { x: 4, y: 5 },
          hotspot: { x: 1, y: 2 },
          image_width: 16,
          image_height: 20,
          scale_factor: 2,
          image_base64: 'CQgH',
        },
      }),
    ).toBe(true);
    expect(
      canToggleCapturedCursor({
        id: 'session-1',
        monitors: [],
        candidates: [],
        captured_cursor: null,
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

  it('uses an unmodified primary-button double click for finishing an annotation gesture', () => {
    expect(
      isFinishAnnotationGestureDoubleClick({
        detail: 2,
        button: 0,
        metaKey: false,
        ctrlKey: false,
        altKey: false,
        shiftKey: false,
      }),
    ).toBe(true);
    expect(
      isFinishAnnotationGestureDoubleClick({
        detail: 1,
        button: 0,
        metaKey: false,
        ctrlKey: false,
        altKey: false,
        shiftKey: false,
      }),
    ).toBe(false);
    expect(
      isFinishAnnotationGestureDoubleClick({
        detail: 2,
        button: 2,
        metaKey: false,
        ctrlKey: false,
        altKey: false,
        shiftKey: false,
      }),
    ).toBe(false);
    expect(
      isFinishAnnotationGestureDoubleClick({
        detail: 2,
        button: 0,
        metaKey: false,
        ctrlKey: false,
        altKey: false,
        shiftKey: true,
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

  it('uses Space for toggling the capture annotation toolbar without stealing Escape', () => {
    expect(
      getCaptureKeyboardToolbarAction(
        {
          key: ' ',
          metaKey: false,
          ctrlKey: false,
          altKey: false,
          shiftKey: false,
        },
        false,
      ),
    ).toBe('toggle');
    expect(
      getCaptureKeyboardToolbarAction(
        {
          key: 'Escape',
          metaKey: false,
          ctrlKey: false,
          altKey: false,
          shiftKey: false,
        },
        true,
      ),
    ).toBeNull();
    expect(
      getCaptureKeyboardToolbarAction(
        {
          key: 'Escape',
          metaKey: false,
          ctrlKey: false,
          altKey: false,
          shiftKey: false,
        },
        false,
      ),
    ).toBeNull();
    expect(
      getCaptureKeyboardToolbarAction(
        {
          key: ' ',
          metaKey: true,
          ctrlKey: false,
          altKey: false,
          shiftKey: false,
        },
        false,
      ),
    ).toBeNull();
    expect(
      getCaptureKeyboardToolbarAction(
        {
          key: 'Enter',
          metaKey: false,
          ctrlKey: false,
          altKey: false,
          shiftKey: false,
        },
        false,
      ),
    ).toBeNull();
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

  it('uses copy shortcuts for completing a hovered capture candidate', () => {
    expect(
      shouldCopyHoverSelectionFromShortcut({
        key: 'Enter',
        metaKey: false,
        ctrlKey: false,
        altKey: false,
        shiftKey: false,
      }),
    ).toBe(true);
    expect(
      shouldCopyHoverSelectionFromShortcut({
        key: 'c',
        metaKey: true,
        ctrlKey: false,
        altKey: false,
        shiftKey: false,
      }),
    ).toBe(true);
    expect(
      shouldCopyHoverSelectionFromShortcut({
        key: 'C',
        metaKey: false,
        ctrlKey: true,
        altKey: false,
        shiftKey: false,
      }),
    ).toBe(true);
    expect(
      shouldCopyHoverSelectionFromShortcut({
        key: ' ',
        metaKey: false,
        ctrlKey: false,
        altKey: false,
        shiftKey: false,
      }),
    ).toBe(false);
    expect(
      shouldCopyHoverSelectionFromShortcut({
        key: ' ',
        metaKey: false,
        ctrlKey: false,
        altKey: false,
        shiftKey: true,
      }),
    ).toBe(false);
  });

  it('does not complete a hovered candidate while drafting a selection', () => {
    expect(
      shouldCopyHoverSelectionFromShortcut(
        {
          key: 'Enter',
          metaKey: false,
          ctrlKey: false,
          altKey: false,
          shiftKey: false,
        },
        { drafting: true },
      ),
    ).toBe(false);
    expect(
      shouldCopyHoverSelectionFromShortcut(
        {
          key: 'c',
          metaKey: true,
          ctrlKey: false,
          altKey: false,
          shiftKey: false,
        },
        { drafting: true },
      ),
    ).toBe(false);
  });

  it('maps successful capture shortcuts to hovered candidate completion actions', () => {
    expect(
      getHoverSelectionCompletionActionFromShortcut({
        key: 'Enter',
        metaKey: false,
        ctrlKey: false,
        altKey: false,
        shiftKey: false,
      }),
    ).toBe('copy');
    expect(
      getHoverSelectionCompletionActionFromShortcut({
        key: 's',
        metaKey: true,
        ctrlKey: false,
        altKey: false,
        shiftKey: false,
      }),
    ).toBe('save');
    expect(
      getHoverSelectionCompletionActionFromShortcut({
        key: 'S',
        metaKey: false,
        ctrlKey: true,
        altKey: false,
        shiftKey: true,
      }),
    ).toBe('quick-save');
    expect(
      getHoverSelectionCompletionActionFromShortcut({
        key: 't',
        metaKey: true,
        ctrlKey: false,
        altKey: false,
        shiftKey: false,
      }),
    ).toBe('pin');
    expect(
      getHoverSelectionCompletionActionFromShortcut({
        key: 'p',
        metaKey: true,
        ctrlKey: false,
        altKey: false,
        shiftKey: false,
      }),
    ).toBe('print');
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

  it('finishes active annotation gestures before dismissing capture layers', () => {
    expect(
      getCancelCapturePointerAction({
        status: 'preview',
        hasSelection: true,
        hasAnnotationGesture: true,
        hasDismissibleLayer: true,
      }),
    ).toBe('finish-annotation');
  });

  it('finishes text annotation edits before dismissing other capture layers', () => {
    expect(
      getCancelCapturePointerAction({
        status: 'preview',
        hasSelection: true,
        hasTextDraft: true,
        hasDismissibleLayer: true,
      }),
    ).toBe('finish-edit');
  });

  it('cancels an active capture when another window is activated', () => {
    expect(shouldCancelCaptureOnBlur({ status: 'selecting' })).toBe(true);
    expect(shouldCancelCaptureOnBlur({ status: 'preview' })).toBe(true);
    expect(shouldCancelCaptureOnBlur({ status: 'idle' })).toBe(false);
    expect(shouldCancelCaptureOnBlur({ status: 'loading' })).toBe(false);
    expect(shouldCancelCaptureOnBlur({ status: 'error' })).toBe(false);
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
