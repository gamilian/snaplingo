import { describe, expect, it } from 'vitest';
import {
  canToggleCapturedCursor,
  getPreviewCaptureCompletionActionFromShortcut,
  getCaptureKeyboardToolbarAction,
  getCaptureSelectionFlowForMode,
  getCancelCapturePointerAction,
  getHoverSelectionCompletionActionFromPointer,
  getSelectionArrowActionFromShortcut,
  getSelectionHistoryStepFromShortcut,
  getUndoRedoActionFromShortcut,
  isCancelCapturePointer,
  isClearAnnotationsShortcut,
  isCopyCaptureKeyboardShortcut,
  isDeleteSelectedAnnotationShortcut,
  getCursorNudgeDeltaFromShortcut,
  getSaveCapturePointerAction,
  getHoverSelectionCompletionActionFromShortcut,
  isMagnifierShortcut,
  isCandidateDetectionModeToggleShortcut,
  isPinCaptureShortcut,
  isPinCapturePointer,
  isPrintCaptureShortcut,
  isRefreshCaptureShortcut,
  isQuickSaveCaptureShortcut,
  isRestoreLastSelectionShortcut,
  isSaveCaptureShortcut,
  isToggleCapturedCursorShortcut,
  shouldCancelCaptureOnBlur,
  shouldRecordSuccessfulCaptureSelection,
} from './captureActions';

describe('capture session actions', () => {

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
    expect(getCaptureSelectionFlowForMode('screenshot-copy')).toBe('copy');
    expect(getCaptureSelectionFlowForMode('screenshot-ocr')).toBe('ocr');
    expect(getCaptureSelectionFlowForMode('silent-screenshot-ocr')).toBe(
      'silent-ocr',
    );
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

  it('uses Cmd/Ctrl+C for copying the current selection', () => {
    expect(
      isCopyCaptureKeyboardShortcut({
        key: 'Enter',
        metaKey: false,
        ctrlKey: false,
      }),
    ).toBe(false);
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

  it('maps preview capture output shortcuts to one completion action', () => {
    expect(
      getPreviewCaptureCompletionActionFromShortcut({
        key: 'c',
        metaKey: true,
        ctrlKey: false,
        altKey: false,
        shiftKey: false,
      }),
    ).toBe('copy');
    expect(
      getPreviewCaptureCompletionActionFromShortcut({
        key: 'Enter',
        metaKey: false,
        ctrlKey: false,
        altKey: false,
        shiftKey: false,
      }),
    ).toBeNull();
    expect(
      getPreviewCaptureCompletionActionFromShortcut({
        key: 's',
        metaKey: true,
        ctrlKey: false,
        altKey: false,
        shiftKey: true,
      }),
    ).toBe('quick-save');
    expect(
      getPreviewCaptureCompletionActionFromShortcut({
        key: 's',
        metaKey: true,
        ctrlKey: false,
        altKey: false,
        shiftKey: false,
      }),
    ).toBe('save');
    expect(
      getPreviewCaptureCompletionActionFromShortcut({
        key: 'p',
        metaKey: true,
        ctrlKey: false,
        altKey: false,
        shiftKey: false,
      }),
    ).toBe('print');
    expect(
      getPreviewCaptureCompletionActionFromShortcut({
        key: 'Enter',
        metaKey: true,
        ctrlKey: false,
        altKey: false,
        shiftKey: false,
      }),
    ).toBeNull();
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

  it('uses the capture mode for hovered candidate Enter completion', () => {
    expect(
      getHoverSelectionCompletionActionFromShortcut(
        {
          key: 'Enter',
          metaKey: false,
          ctrlKey: false,
          altKey: false,
          shiftKey: false,
        },
        { mode: 'screenshot-ocr' },
      ),
    ).toBe('ocr');
    expect(
      getHoverSelectionCompletionActionFromShortcut(
        {
          key: 'Enter',
          metaKey: false,
          ctrlKey: false,
          altKey: false,
          shiftKey: false,
        },
        { mode: 'screenshot-translate' },
      ),
    ).toBe('ocr-translate');
    expect(
      getHoverSelectionCompletionActionFromShortcut(
        {
          key: 'c',
          metaKey: true,
          ctrlKey: false,
          altKey: false,
          shiftKey: false,
        },
        { mode: 'screenshot-translate' },
      ),
    ).toBe('copy');
  });

  it('uses the capture mode for hovered candidate primary click completion', () => {
    const event = {
      detail: 1,
      button: 0,
      metaKey: false,
      ctrlKey: false,
      altKey: false,
      shiftKey: false,
    };

    expect(getHoverSelectionCompletionActionFromPointer(event)).toBe('copy');
    expect(
      getHoverSelectionCompletionActionFromPointer(event, {
        mode: 'screenshot-ocr',
      }),
    ).toBe('ocr');
    expect(
      getHoverSelectionCompletionActionFromPointer(event, {
        mode: 'screenshot-translate',
      }),
    ).toBe('ocr-translate');
    expect(
      getHoverSelectionCompletionActionFromPointer({
        ...event,
        metaKey: true,
      }),
    ).toBeNull();
  });

  it('uses plain Tab to toggle window and interface-element detection', () => {
    expect(
      isCandidateDetectionModeToggleShortcut({
        key: 'Tab',
        metaKey: false,
        ctrlKey: false,
        altKey: false,
        shiftKey: false,
      }),
    ).toBe(true);
    expect(
      isCandidateDetectionModeToggleShortcut({
        key: 'Tab',
        metaKey: false,
        ctrlKey: false,
        altKey: false,
        shiftKey: true,
      }),
    ).toBe(false);
    expect(
      isCandidateDetectionModeToggleShortcut({
        key: 'Tab',
        metaKey: true,
        ctrlKey: false,
        altKey: false,
        shiftKey: false,
      }),
    ).toBe(false);
    expect(
      isCandidateDetectionModeToggleShortcut({
        key: 'Enter',
        metaKey: false,
        ctrlKey: false,
        altKey: false,
        shiftKey: false,
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

  it('keeps the capture session alive when the window blurs during output', () => {
    expect(
      shouldCancelCaptureOnBlur({
        status: 'preview',
        isRenderingOutput: true,
      }),
    ).toBe(false);
  });
});
