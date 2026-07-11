import type { ReactElement, ReactNode } from 'react';
import { describe, expect, it, vi } from 'vitest';

import type { CaptureWorkspaceRuntimeActions } from '../../application/capture-workspace/types';
import { DEFAULT_ANNOTATION_STYLE } from './annotationStyle';
import { CaptureEditorToolbar } from './captureEditorToolbar';
import {
  CaptureWorkspaceView,
  type CaptureWorkspaceViewRenderState,
} from './CaptureWorkspaceView';

const selection = { x: 120, y: 230, width: 160, height: 90 };

describe('CaptureWorkspaceView runtime seam', () => {
  it('renders runtime state and sends user actions through runtime actions', () => {
    const actions = createActions();
    const renderState = {
      status: 'preview',
      mode: 'screenshot',
      session: null,
      sessionId: 'view-session',
      cursorPoint: null,
      startPoint: null,
      selection,
      hoverSelection: null,
      previewImageBase64: 'preview-image',
      editGesture: null,
      activeAnnotationTool: null,
      annotationGesture: null,
      draftAnnotation: null,
      selectedAnnotationIndex: null,
      annotationMoveGesture: null,
      draftSelectionMoveGesture: null,
      textDraft: null,
      textDraftAnnotationIndex: null,
      annotationStyle: DEFAULT_ANNOTATION_STYLE,
      textFontSize: 18,
      annotationHistory: { annotations: [], undoneAnnotations: [] },
      isAnnotationToolbarVisible: true,
      cursorColor: null,
      colorSampleFormat: 'hex',
      isMagnifierRequested: false,
      includeCapturedCursor: false,
      isRenderingOutput: false,
      hasHydratedPixelSource: false,
      error: null,
      annotations: [],
      selectedAnnotation: null,
      hasAnnotationEditingContext: false,
      canUndoAnnotation: false,
      canRedoAnnotation: false,
      isTextSizingActive: false,
      isFillModeActive: false,
      captureCandidates: [],
      areCaptureImagesReady: true,
      snapTargetRects: [],
      selectionBounds: { x: 100, y: 200, width: 800, height: 600 },
      viewportBounds: { x: 0, y: 0, width: 800, height: 600 },
      selectionViewportRect: { x: 20, y: 30, width: 160, height: 90 },
      cursorViewportPoint: null,
      selectedAnnotationBounds: null,
      toolbarPosition: { x: 20, y: 134 },
      toolbarWidth: 640,
      cursorMonitor: null,
      cursorInMonitorPoint: null,
      shouldTrackMagnifierCursor: false,
      isMagnifierShown: false,
      magnifierSelection: selection,
      textDraftInputRef: { current: null },
      selectionOverlayCanvasRef: { current: null },
      selectionOverlayCssSize: { width: 800, height: 600 },
      selectionOverlayPixelRatio: 1,
    } satisfies CaptureWorkspaceViewRenderState;
    const runtime = { renderState, actions };
    const view = CaptureWorkspaceView(runtime);
    const root = view as ReactElement;

    const pointerEvent = {
      clientX: 45,
      clientY: 65,
      pointerId: 7,
      button: 0,
      detail: 1,
      metaKey: false,
      ctrlKey: false,
      altKey: false,
      shiftKey: false,
      currentTarget: { setPointerCapture: vi.fn() },
      preventDefault: vi.fn(),
      stopPropagation: vi.fn(),
    };
    root.props.onPointerDown(pointerEvent);

    expect(pointerEvent.currentTarget.setPointerCapture).toHaveBeenCalledWith(7);
    expect(actions.pointerDown).toHaveBeenCalledWith({
      point: { x: 145, y: 265 },
      button: 0,
      shiftKey: false,
      source: 'root',
    });

    const toolbar = findElement(root, CaptureEditorToolbar);
    toolbar.props.onToggleAnnotationTool('rectangle');
    toolbar.props.onCopy();

    expect(actions.toggleAnnotationTool).toHaveBeenCalledWith('rectangle');
    expect(actions.completePreviewSelection).toHaveBeenCalledWith(
      'copy',
      selection,
    );
  });
});

function createActions() {
  return {
    connectHost: vi.fn(async () => () => undefined),
    updateHostReadiness: vi.fn(async () => undefined),
    startSession: vi.fn(async () => undefined),
    refreshSession: vi.fn(async () => undefined),
    cancelSession: vi.fn(async () => undefined),
    renderSelectionPreview: vi.fn(async () => undefined),
    completeCandidateSelection: vi.fn(async () => undefined),
    completeManualSelection: vi.fn(async () => undefined),
    completePreviewSelection: vi.fn(async () => undefined),
    resetPreview: vi.fn(),
    pointerDown: vi.fn(() => true),
    pointerMove: vi.fn(() => true),
    pointerUp: vi.fn(async () => true),
    resizePointerDown: vi.fn(() => true),
    wheel: vi.fn(() => true),
    commitTextDraft: vi.fn(),
    updateTextDraftText: vi.fn(),
    discardTextDraft: vi.fn(),
    selectMoveTool: vi.fn(),
    toggleAnnotationTool: vi.fn(),
    applySelectedAnnotationStyle: vi.fn(),
    updateTextDraftFontSize: vi.fn(),
    updateCursorColor: vi.fn(),
    updatePolledCursor: vi.fn(),
    updatePolledHover: vi.fn(),
    keyDown: vi.fn(() => true),
    hydrateSnapshots: vi.fn(async () => undefined),
  } satisfies CaptureWorkspaceRuntimeActions;
}

function findElement(
  node: ReactNode,
  type: ReactElement['type'],
): ReactElement {
  if (isReactElement(node)) {
    if (node.type === type) return node;
    const found = findElementOrNull(node.props.children, type);
    if (found) return found;
  } else if (Array.isArray(node)) {
    for (const child of node) {
      const found = findElementOrNull(child, type);
      if (found) return found;
    }
  }
  throw new Error('Expected element was not rendered');
}

function findElementOrNull(
  node: ReactNode,
  type: ReactElement['type'],
): ReactElement | null {
  try {
    return findElement(node, type);
  } catch {
    return null;
  }
}

function isReactElement(node: ReactNode): node is ReactElement {
  return Boolean(node && typeof node === 'object' && 'type' in node);
}
