// @vitest-environment happy-dom

import { act, createElement, type RefObject } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { afterEach, describe, expect, expectTypeOf, it, vi } from 'vitest';

import { DEFAULT_ANNOTATION_STYLE } from './annotationStyle';
import {
  CaptureWorkspaceView,
  type CaptureWorkspaceViewActions,
  type CaptureWorkspaceViewRenderState,
} from './CaptureWorkspaceView';

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT =
  true;

const selection = { x: 120, y: 230, width: 160, height: 90 };
const mountedRoots: Array<{ container: HTMLDivElement; root: Root }> = [];

afterEach(() => {
  for (const mounted of mountedRoots.splice(0)) {
    act(() => mounted.root.unmount());
    mounted.container.remove();
  }
  delete (HTMLElement.prototype as { setPointerCapture?: unknown })
    .setPointerCapture;
});

describe('CaptureWorkspaceView runtime seam', () => {
  it('exposes only presentation state and user-triggered actions', () => {
    type ExpectedRenderStateKeys =
      | 'status'
      | 'error'
      | 'viewportBounds'
      | 'selectionBounds'
      | 'isRenderingOutput'
      | 'editor'
      | 'toolbar'
      | 'dom'
      | 'magnifier';
    type ExpectedActionKeys =
      | 'pointerDown'
      | 'pointerMove'
      | 'pointerUp'
      | 'resizePointerDown'
      | 'wheel'
      | 'commitTextDraft'
      | 'updateTextDraftText'
      | 'discardTextDraft'
      | 'selectMoveTool'
      | 'toggleAnnotationTool'
      | 'applySelectedAnnotationStyle'
      | 'updateTextDraftFontSize'
      | 'cancelSession'
      | 'completePreviewSelection';

    expectTypeOf<keyof CaptureWorkspaceViewRenderState>().toEqualTypeOf<
      ExpectedRenderStateKeys
    >();
    expectTypeOf<keyof CaptureWorkspaceViewActions>().toEqualTypeOf<
      ExpectedActionKeys
    >();
    expectTypeOf<
      keyof CaptureWorkspaceViewRenderState['editor']
    >().toEqualTypeOf<
      | 'selection'
      | 'selectionViewportRect'
      | 'previewImageBase64'
      | 'draftAnnotation'
      | 'textDraft'
      | 'annotationStyle'
      | 'selectedAnnotationBounds'
      | 'activeAnnotationTool'
    >();
    expectTypeOf<
      keyof CaptureWorkspaceViewRenderState['toolbar']
    >().toEqualTypeOf<
      | 'position'
      | 'width'
      | 'isVisible'
      | 'textFontSize'
      | 'isTextSizingActive'
      | 'isFillModeActive'
    >();
    expectTypeOf<keyof CaptureWorkspaceViewRenderState['dom']>().toEqualTypeOf<
      'textDraftInputRef' | 'selectionOverlay'
    >();
    expectTypeOf<
      keyof CaptureWorkspaceViewRenderState['dom']['selectionOverlay']
    >().toEqualTypeOf<'canvasRef' | 'cssSize' | 'pixelRatio'>();
    expectTypeOf<
      keyof CaptureWorkspaceViewRenderState['magnifier']
    >().toEqualTypeOf<
      | 'isShown'
      | 'cursorMonitor'
      | 'cursorViewportPoint'
      | 'cursorInMonitorPoint'
      | 'selection'
      | 'cursorColor'
      | 'colorSampleFormat'
    >();

    if (false) {
      const renderState = {} as CaptureWorkspaceViewRenderState;
      const actions = {} as CaptureWorkspaceViewActions;
      // @ts-expect-error host lifecycle state is outside the View contract
      void renderState.session;
      // @ts-expect-error host lifecycle actions are outside the View contract
      void actions.connectHost();
      // @ts-expect-error polling actions are outside the View contract
      actions.updatePolledCursor({ x: 0, y: 0 });
    }
  });

  it('routes mounted pointer, wheel, preview, and toolbar interactions through View actions', async () => {
    const actions = createActions();
    const runtime = {
      renderState: createRenderState(),
      actions,
    };
    const pointerCapture = vi.fn();
    Object.defineProperty(HTMLElement.prototype, 'setPointerCapture', {
      configurable: true,
      value: pointerCapture,
    });
    const container = document.createElement('div');
    document.body.append(container);
    const root = createRoot(container);
    mountedRoots.push({ container, root });

    await act(async () => {
      root.render(createElement(CaptureWorkspaceView, runtime));
    });

    const workspace = container.firstElementChild as HTMLDivElement;
    await act(async () => {
      workspace.dispatchEvent(
        new PointerEvent('pointerdown', {
          bubbles: true,
          cancelable: true,
          clientX: 45,
          clientY: 65,
          pointerId: 7,
          button: 0,
        }),
      );
      workspace.dispatchEvent(
        new PointerEvent('pointermove', {
          bubbles: true,
          clientX: 55,
          clientY: 75,
          pointerId: 7,
          button: 0,
        }),
      );
      workspace.dispatchEvent(
        new PointerEvent('pointerup', {
          bubbles: true,
          clientX: 65,
          clientY: 85,
          pointerId: 7,
          button: 0,
        }),
      );
    });

    expect(pointerCapture).toHaveBeenCalledWith(7);
    expect(actions.pointerDown).toHaveBeenCalledWith({
      point: { x: 145, y: 265 },
      button: 0,
      shiftKey: false,
      source: 'root',
    });
    expect(actions.pointerMove).toHaveBeenCalledWith({
      point: { x: 155, y: 275 },
      button: 0,
      shiftKey: false,
      source: 'root',
    });
    expect(actions.pointerUp).toHaveBeenCalledWith({
      point: { x: 165, y: 285 },
      button: 0,
      shiftKey: false,
      source: 'root',
    });

    const wheel = new WheelEvent('wheel', {
      bubbles: true,
      cancelable: true,
      deltaY: 80,
    });
    await act(async () => {
      workspace.dispatchEvent(wheel);
    });
    expect(actions.wheel).toHaveBeenCalledWith({
      deltaY: 80,
      metaKey: false,
      ctrlKey: false,
      altKey: false,
    });
    expect(wheel.defaultPrevented).toBe(true);

    actions.pointerDown.mockClear();
    pointerCapture.mockClear();
    const preview = Array.from(container.querySelectorAll('div')).find(
      (element) =>
        element.classList.contains('rounded-[8px]') &&
        element.classList.contains('bg-transparent') &&
        element.style.left === '20px',
    );
    expect(preview).not.toBeNull();
    await act(async () => {
      preview!.dispatchEvent(
        new PointerEvent('pointerdown', {
          bubbles: true,
          cancelable: true,
          clientX: 75,
          clientY: 95,
          pointerId: 9,
          button: 0,
          detail: 1,
        }),
      );
    });
    expect(pointerCapture).toHaveBeenCalledTimes(1);
    expect(pointerCapture).toHaveBeenCalledWith(9);
    expect(actions.pointerDown).toHaveBeenCalledTimes(1);
    expect(actions.pointerDown).toHaveBeenCalledWith({
      point: { x: 175, y: 295 },
      button: 0,
      detail: 1,
      metaKey: false,
      ctrlKey: false,
      altKey: false,
      shiftKey: false,
      source: 'preview',
    });

    const copyButton = container.querySelector<HTMLButtonElement>(
      'button[aria-label="Copy selection"]',
    );
    expect(copyButton).not.toBeNull();
    await act(async () => copyButton!.click());
    expect(actions.completePreviewSelection).toHaveBeenCalledWith(
      'copy',
      selection,
    );
  });

  it('keeps a restored preview usable while showing its replacement error', async () => {
    const container = document.createElement('div');
    document.body.append(container);
    const root = createRoot(container);
    mountedRoots.push({ container, root });
    const renderState = {
      ...createRenderState(),
      error: 'replacement load failed',
    };

    await act(async () => {
      root.render(
        createElement(CaptureWorkspaceView, {
          renderState,
          actions: createActions(),
        }),
      );
    });

    expect(container.textContent).toContain('replacement load failed');
    expect(
      container.querySelector('button[aria-label="Copy selection"]'),
    ).not.toBeNull();
  });
});

function createRenderState(): CaptureWorkspaceViewRenderState {
  const textDraftInputRef: RefObject<HTMLTextAreaElement> = {
    current: null,
  };
  const canvasRef: RefObject<HTMLCanvasElement> = { current: null };

  return {
    status: 'preview',
    error: null,
    viewportBounds: { x: 0, y: 0, width: 800, height: 600 },
    selectionBounds: { x: 100, y: 200, width: 800, height: 600 },
    isRenderingOutput: false,
    editor: {
      selection,
      selectionViewportRect: { x: 20, y: 30, width: 160, height: 90 },
      previewImageBase64: 'preview-image',
      draftAnnotation: null,
      textDraft: null,
      annotationStyle: DEFAULT_ANNOTATION_STYLE,
      selectedAnnotationBounds: null,
      activeAnnotationTool: null,
    },
    toolbar: {
      position: { x: 20, y: 134 },
      width: 640,
      isVisible: true,
      textFontSize: 18,
      isTextSizingActive: false,
      isFillModeActive: false,
    },
    dom: {
      textDraftInputRef,
      selectionOverlay: {
        canvasRef,
        cssSize: { width: 800, height: 600 },
        pixelRatio: 1,
      },
    },
    magnifier: {
      isShown: false,
      cursorMonitor: null,
      cursorViewportPoint: null,
      cursorInMonitorPoint: null,
      selection,
      cursorColor: null,
      colorSampleFormat: 'hex',
    },
  };
}

function createActions() {
  return {
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
    cancelSession: vi.fn(async () => undefined),
    completePreviewSelection: vi.fn(async () => undefined),
  } satisfies CaptureWorkspaceViewActions;
}
