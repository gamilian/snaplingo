import { describe, expect, it } from 'vitest';

import { emptyAnnotationHistory } from './annotationHistory';
import { createInitialCaptureWorkspaceState } from './captureWorkspaceState';
import { getCaptureWorkspaceDerivedState } from './captureWorkspaceDerived';
import type { CaptureSessionView } from './types';

function createSession(): CaptureSessionView {
  return {
    id: 'session-1',
    monitors: [
      {
        id: 'primary',
        logical_bounds: { x: -100, y: 20, width: 300, height: 200 },
        physical_bounds: { x: -200, y: 40, width: 600, height: 400 },
        scale_factor: 2,
        image_base64: 'monitor-png',
      },
    ],
    candidates: [
      {
        id: 'window-1',
        kind: 'window',
        rect: { x: -50, y: 60, width: 120, height: 80 },
        priority: 10,
      },
    ],
    captured_cursor: {
      logical_position: { x: 10, y: 40 },
      hotspot: { x: 0, y: 0 },
      image_width: 16,
      image_height: 16,
      scale_factor: 1,
      image_base64: 'cursor-png',
    },
  };
}

describe('captureWorkspaceDerived', () => {
  it('derives capture geometry from the active session', () => {
    const session = createSession();
    const state = {
      ...createInitialCaptureWorkspaceState(),
      status: 'preview' as const,
      session,
      selection: { x: -50, y: 60, width: 120, height: 80 },
      cursorPoint: { x: 10, y: 40 },
      isMagnifierRequested: true,
    };

    const derived = getCaptureWorkspaceDerivedState({
      state,
      hydratedCaptureSessionId: session.id,
      toolbarGap: 14,
      toolbarSize: { width: 640, height: 42 },
    });

    expect(derived.areCaptureImagesReady).toBe(true);
    expect(derived.captureCandidates.map((candidate) => candidate.id)).toEqual([
      'monitor:primary',
      'window-1',
    ]);
    expect(derived.selectionBounds).toEqual({
      x: -100,
      y: 20,
      width: 300,
      height: 200,
    });
    expect(derived.viewportBounds).toEqual({
      x: 0,
      y: 0,
      width: 300,
      height: 200,
    });
    expect(derived.selectionViewportRect).toEqual({
      x: 50,
      y: 40,
      width: 120,
      height: 80,
    });
    expect(derived.cursorViewportPoint).toEqual({ x: 110, y: 20 });
    expect(derived.snapTargetRects).toEqual([
      session.monitors[0].logical_bounds,
      session.candidates[0].rect,
    ]);
    expect(derived.toolbarPosition).not.toBeNull();
  });

  it('derives annotation and toolbar state from editor state', () => {
    const session = createSession();
    const annotation = {
      type: 'rectangle' as const,
      rect: { x: 2, y: 4, width: 30, height: 20 },
      color: [255, 0, 0, 255] as [number, number, number, number],
      stroke_width: 2,
      filled: false,
    };
    const state = {
      ...createInitialCaptureWorkspaceState(),
      status: 'preview' as const,
      session,
      selection: { x: -50, y: 60, width: 120, height: 80 },
      activeAnnotationTool: 'ellipse' as const,
      selectedAnnotationIndex: 0,
      annotationHistory: {
        ...emptyAnnotationHistory(),
        annotations: [annotation],
        undoSnapshots: [[annotation]],
      },
    };

    const derived = getCaptureWorkspaceDerivedState({
      state,
      hydratedCaptureSessionId: null,
      toolbarGap: 14,
      toolbarSize: { width: 640, height: 42 },
    });

    expect(derived.selectedAnnotation).toBe(annotation);
    expect(derived.hasAnnotationEditingContext).toBe(true);
    expect(derived.canUndoAnnotation).toBe(true);
    expect(derived.canRedoAnnotation).toBe(false);
    expect(derived.isFillModeActive).toBe(true);
    expect(derived.selectedAnnotationBounds).toEqual(annotation.rect);
  });
});
