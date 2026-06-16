import { describe, expect, it } from 'vitest';
import type { AnnotationCommand, LogicalRect } from './types';
import {
  type CaptureInvoke,
  type CaptureInvokeArgs,
  isSaveCaptureShortcut,
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
});
