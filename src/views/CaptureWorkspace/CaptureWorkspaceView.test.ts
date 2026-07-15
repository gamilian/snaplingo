// @vitest-environment happy-dom

import { act, createElement, type RefObject } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, describe, expect, expectTypeOf, it, vi } from "vitest";

import { DEFAULT_ANNOTATION_STYLE } from "./annotationStyle";
import {
  CaptureWorkspaceView,
  type CaptureWorkspaceViewActions,
  type CaptureWorkspaceViewRenderState,
} from "./CaptureWorkspaceView";

(
  globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }
).IS_REACT_ACT_ENVIRONMENT = true;

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

describe("CaptureWorkspaceView runtime seam", () => {
  it("exposes only presentation state and user-triggered actions", () => {
    type ExpectedRenderStateKeys =
      | "status"
      | "error"
      | "viewportBounds"
      | "selectionBounds"
      | "isRenderingOutput"
      | "silentOcrHint"
      | "editor"
      | "toolbar"
      | "dom"
      | "magnifier";
    type ExpectedActionKeys =
      | "pointerDown"
      | "pointerMove"
      | "pointerUp"
      | "resizePointerDown"
      | "resizeAnnotationPointerDown"
      | "wheel"
      | "commitTextDraft"
      | "updateTextDraftText"
      | "discardTextDraft"
      | "selectMoveTool"
      | "toggleAnnotationTool"
      | "applySelectedAnnotationStyle"
      | "updateTextDraftFontSize"
      | "commitAnnotationSizeDefault"
      | "undoAnnotation"
      | "redoAnnotation"
      | "cancelSession"
      | "completePreviewSelection";

    expectTypeOf<
      keyof CaptureWorkspaceViewRenderState
    >().toEqualTypeOf<ExpectedRenderStateKeys>();
    expectTypeOf<
      keyof CaptureWorkspaceViewActions
    >().toEqualTypeOf<ExpectedActionKeys>();
    expectTypeOf<
      keyof CaptureWorkspaceViewRenderState["editor"]
    >().toEqualTypeOf<
      | "selection"
      | "selectionViewportRect"
      | "previewImageBase64"
      | "annotations"
      | "draftAnnotation"
      | "textDraft"
      | "annotationStyle"
      | "selectedAnnotationBounds"
      | "activeAnnotationTool"
    >();
    expectTypeOf<
      keyof CaptureWorkspaceViewRenderState["toolbar"]
    >().toEqualTypeOf<
      | "position"
      | "width"
      | "isVisible"
      | "textFontSize"
      | "isTextSizingActive"
      | "isFillModeActive"
      | "canUndo"
      | "canRedo"
    >();
    expectTypeOf<keyof CaptureWorkspaceViewRenderState["dom"]>().toEqualTypeOf<
      "textDraftInputRef" | "selectionOverlay"
    >();
    expectTypeOf<
      keyof CaptureWorkspaceViewRenderState["dom"]["selectionOverlay"]
    >().toEqualTypeOf<"canvasRef" | "cssSize" | "pixelRatio">();
    expectTypeOf<
      keyof CaptureWorkspaceViewRenderState["magnifier"]
    >().toEqualTypeOf<
      | "isShown"
      | "cursorMonitor"
      | "cursorViewportPoint"
      | "cursorInMonitorPoint"
      | "selection"
      | "cursorColor"
      | "colorSampleFormat"
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

  it("routes mounted pointer, wheel, preview, and toolbar interactions through View actions", async () => {
    const actions = createActions();
    const runtime = {
      renderState: {
        ...createRenderState(),
        editor: {
          ...createRenderState().editor,
          selectedAnnotationBounds: { x: 10, y: 10, width: 40, height: 30 },
        },
      },
      actions,
    };
    const pointerCapture = vi.fn();
    Object.defineProperty(HTMLElement.prototype, "setPointerCapture", {
      configurable: true,
      value: pointerCapture,
    });
    const container = document.createElement("div");
    document.body.append(container);
    const root = createRoot(container);
    mountedRoots.push({ container, root });

    await act(async () => {
      root.render(createElement(CaptureWorkspaceView, runtime));
    });

    const workspace = container.firstElementChild as HTMLDivElement;
    await act(async () => {
      workspace.dispatchEvent(
        new PointerEvent("pointerdown", {
          bubbles: true,
          cancelable: true,
          clientX: 45,
          clientY: 65,
          pointerId: 7,
          button: 0,
        }),
      );
      workspace.dispatchEvent(
        new PointerEvent("pointermove", {
          bubbles: true,
          clientX: 55,
          clientY: 75,
          pointerId: 7,
          button: 0,
        }),
      );
      workspace.dispatchEvent(
        new PointerEvent("pointerup", {
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
      source: "root",
    });
    expect(actions.pointerMove).toHaveBeenCalledWith({
      point: { x: 155, y: 275 },
      button: 0,
      shiftKey: false,
      source: "root",
    });
    expect(actions.pointerUp).toHaveBeenCalledWith({
      point: { x: 165, y: 285 },
      button: 0,
      shiftKey: false,
      source: "root",
    });

    const wheel = new WheelEvent("wheel", {
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
    const preview = Array.from(container.querySelectorAll("div")).find(
      (element) =>
        element.classList.contains("rounded-[8px]") &&
        element.classList.contains("bg-transparent") &&
        element.style.left === "20px",
    );
    expect(preview).not.toBeNull();
    await act(async () => {
      preview!.dispatchEvent(
        new PointerEvent("pointerdown", {
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
      source: "preview",
    });

    const shapesButton = container.querySelector<HTMLButtonElement>(
      'button[aria-label="Shapes"]',
    );
    expect(shapesButton).not.toBeNull();
    await act(async () => shapesButton!.click());
    expect(actions.toggleAnnotationTool).toHaveBeenCalledWith("rectangle");
    expect(
      container.querySelector('[role="menu"][aria-label="Shapes"]'),
    ).not.toBeNull();
    const ellipseButton = container.querySelector<HTMLButtonElement>(
      'button[aria-label="Draw ellipse or circle annotation"]',
    );
    expect(ellipseButton).not.toBeNull();
    await act(async () => ellipseButton!.click());
    expect(actions.toggleAnnotationTool).toHaveBeenCalledWith("ellipse");
    expect(
      container.querySelector('[role="menu"][aria-label="Shapes"]'),
    ).toBeNull();

    const arrowButton = container.querySelector<HTMLButtonElement>(
      'button[aria-label="Arrows and lines"]',
    );
    await act(async () => arrowButton!.click());
    expect(actions.toggleAnnotationTool).toHaveBeenCalledWith("arrow");
    const northEdge = container.querySelector<HTMLButtonElement>(
      'button[aria-label="Resize selection edge n"]',
    );
    expect(northEdge).not.toBeNull();
    await act(async () => {
      northEdge!.dispatchEvent(
        new PointerEvent("pointerdown", {
          bubbles: true,
          cancelable: true,
          clientX: 80,
          clientY: 30,
          pointerId: 11,
          button: 0,
        }),
      );
    });
    expect(actions.resizePointerDown).toHaveBeenCalledWith(
      "n",
      expect.objectContaining({ source: "preview" }),
    );

    const annotationEastHandle = container.querySelector<HTMLButtonElement>(
      'button[aria-label="Resize annotation e"]',
    );
    expect(annotationEastHandle).not.toBeNull();
    expect(annotationEastHandle!.className).toContain("cursor-ew-resize");
    await act(async () => {
      annotationEastHandle!.dispatchEvent(
        new PointerEvent("pointerdown", {
          bubbles: true,
          cancelable: true,
          clientX: 70,
          clientY: 55,
          pointerId: 12,
          button: 0,
        }),
      );
    });
    expect(actions.resizeAnnotationPointerDown).toHaveBeenCalledWith(
      "e",
      expect.objectContaining({ source: "preview" }),
    );
    const annotationEastEdge = container.querySelector<HTMLButtonElement>(
      'button[aria-label="Resize annotation edge e"]',
    );
    expect(annotationEastEdge).not.toBeNull();
    expect(annotationEastEdge!.className).toContain("cursor-ew-resize");

    const undoButton = container.querySelector<HTMLButtonElement>(
      'button[aria-label="Undo annotation"]',
    );
    const redoButton = container.querySelector<HTMLButtonElement>(
      'button[aria-label="Redo annotation"]',
    );
    const pinButton = container.querySelector<HTMLButtonElement>(
      'button[aria-label="Pin capture"]',
    );
    expect(undoButton).not.toBeNull();
    expect(redoButton).not.toBeNull();
    expect(pinButton).not.toBeNull();
    await act(async () => {
      undoButton!.click();
      redoButton!.click();
      pinButton!.click();
    });
    expect(actions.undoAnnotation).toHaveBeenCalledOnce();
    expect(actions.redoAnnotation).toHaveBeenCalledOnce();
    expect(actions.completePreviewSelection).toHaveBeenCalledWith(
      "pin",
      selection,
    );

    const saveButton = container.querySelector<HTMLButtonElement>(
      'button[aria-label="Save selection"]',
    );
    expect(saveButton).not.toBeNull();
    await act(async () => saveButton!.click());
    expect(actions.completePreviewSelection).toHaveBeenCalledWith(
      "save",
      selection,
    );
    const copyButton = container.querySelector<HTMLButtonElement>(
      'button[aria-label="Copy selection"]',
    );
    expect(copyButton).not.toBeNull();
    await act(async () => copyButton!.click());
    expect(actions.completePreviewSelection).toHaveBeenCalledWith(
      "copy",
      selection,
    );

    const sizeRange = container.querySelector<HTMLInputElement>(
      'input[aria-label="Annotation stroke width"]',
    );
    expect(sizeRange).not.toBeNull();
    await act(async () => {
      sizeRange!.value = "5";
      sizeRange!.dispatchEvent(new Event("input", { bubbles: true }));
      sizeRange!.dispatchEvent(new PointerEvent("pointerup", { bubbles: true }));
    });
    expect(actions.commitAnnotationSizeDefault).toHaveBeenCalledWith(
      "stroke",
      5,
    );
  });

  it("opens the color palette and manages preset colors", async () => {
    const actions = createActions();
    const updateAnnotationColorPresets = vi.fn();
    const container = document.createElement("div");
    document.body.append(container);
    const root = createRoot(container);
    mountedRoots.push({ container, root });

    await act(async () => {
      root.render(
        createElement(CaptureWorkspaceView, {
          renderState: createRenderState(),
          actions,
          annotationColorPresets: [
            [255, 77, 79, 255],
            [24, 144, 255, 255],
          ],
          onUpdateAnnotationColorPresets: updateAnnotationColorPresets,
        }),
      );
    });

    const colorButton = container.querySelector<HTMLButtonElement>(
      'button[aria-label="Annotation color"]',
    );
    expect(colorButton).not.toBeNull();
    await act(async () => colorButton!.click());
    expect(
      container.querySelector(
        '[role="dialog"][aria-label="Annotation colors"]',
      ),
    ).not.toBeNull();

    const bluePreset = container.querySelector<HTMLButtonElement>(
      'button[aria-label="Use preset #1890FF"]',
    );
    await act(async () => bluePreset!.click());
    expect(actions.applySelectedAnnotationStyle).toHaveBeenLastCalledWith(
      expect.objectContaining({ color: [24, 144, 255, 255] }),
      18,
    );

    const palette = container.querySelector<HTMLInputElement>(
      'input[aria-label="Color palette"]',
    );
    expect(palette!.className).toContain("absolute");
    expect(palette!.className).toContain("inset-0");
    expect(palette!.className).not.toContain("sr-only");
    const paletteClick = vi.spyOn(palette!, "click");
    await act(async () => {
      container
        .querySelector<HTMLButtonElement>(
          'button[aria-label="Open system color palette"]',
        )!
        .click();
    });
    expect(paletteClick).toHaveBeenCalledOnce();
    await act(async () => {
      palette!.value = "#663399";
      palette!.dispatchEvent(new Event("input", { bubbles: true }));
    });
    expect(actions.applySelectedAnnotationStyle).toHaveBeenLastCalledWith(
      expect.objectContaining({ color: [102, 51, 153, 255] }),
      18,
    );

    await act(async () => {
      container
        .querySelector<HTMLButtonElement>(
          'button[aria-label="Add preset color"]',
        )!
        .click();
    });
    expect(updateAnnotationColorPresets).toHaveBeenLastCalledWith([
      [255, 77, 79, 255],
      [24, 144, 255, 255],
      [102, 51, 153, 255],
    ]);

    await act(async () => {
      palette!.value = "#FF8800";
      palette!.dispatchEvent(new Event("input", { bubbles: true }));
      container
        .querySelector<HTMLButtonElement>(
          'button[aria-label="Replace selected preset"]',
        )!
        .click();
    });
    expect(updateAnnotationColorPresets).toHaveBeenLastCalledWith([
      [255, 77, 79, 255],
      [24, 144, 255, 255],
      [255, 136, 0, 255],
    ]);

    await act(async () => {
      container
        .querySelector<HTMLButtonElement>(
          'button[aria-label="Delete selected preset"]',
        )!
        .click();
    });
    expect(updateAnnotationColorPresets).toHaveBeenLastCalledWith([
      [255, 77, 79, 255],
      [24, 144, 255, 255],
    ]);
  });

  it("rolls back an optimistic preset change when persistence fails", async () => {
    const actions = createActions();
    const updateAnnotationColorPresets = vi
      .fn()
      .mockRejectedValue(new Error("database unavailable"));
    const container = document.createElement("div");
    document.body.append(container);
    const root = createRoot(container);
    mountedRoots.push({ container, root });

    await act(async () => {
      root.render(
        createElement(CaptureWorkspaceView, {
          renderState: createRenderState(),
          actions,
          annotationColorPresets: [[255, 77, 79, 255]],
          onUpdateAnnotationColorPresets: updateAnnotationColorPresets,
        }),
      );
    });
    await act(async () => {
      container
        .querySelector<HTMLButtonElement>(
          'button[aria-label="Annotation color"]',
        )!
        .click();
    });
    const palette = container.querySelector<HTMLInputElement>(
      'input[aria-label="Color palette"]',
    )!;
    await act(async () => {
      palette.value = "#663399";
      palette.dispatchEvent(new Event("input", { bubbles: true }));
    });
    await act(async () => {
      container
        .querySelector<HTMLButtonElement>(
          'button[aria-label="Add preset color"]',
        )!
        .click();
    });

    expect(container.querySelector('[role="alert"]')?.textContent).toContain(
      "颜色预设保存失败",
    );
    expect(
      container.querySelector('button[aria-label="Use preset #663399"]'),
    ).toBeNull();
  });

  it("keeps a restored preview usable while showing its replacement error", async () => {
    const container = document.createElement("div");
    document.body.append(container);
    const root = createRoot(container);
    mountedRoots.push({ container, root });
    const renderState = {
      ...createRenderState(),
      error: "replacement load failed",
    };

    await act(async () => {
      root.render(
        createElement(CaptureWorkspaceView, {
          renderState,
          actions: createActions(),
        }),
      );
    });

    expect(container.textContent).toContain("replacement load failed");
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
    status: "preview",
    error: null,
    viewportBounds: { x: 0, y: 0, width: 800, height: 600 },
    selectionBounds: { x: 100, y: 200, width: 800, height: 600 },
    isRenderingOutput: false,
    silentOcrHint: null,
    editor: {
      selection,
      selectionViewportRect: { x: 20, y: 30, width: 160, height: 90 },
      previewImageBase64: "preview-image",
      annotations: [],
      draftAnnotation: null,
      textDraft: null,
      annotationStyle: DEFAULT_ANNOTATION_STYLE,
      selectedAnnotationBounds: null,
      activeAnnotationTool: null,
    },
    toolbar: {
      position: { x: 20, y: 134 },
      width: 700,
      isVisible: true,
      textFontSize: 18,
      isTextSizingActive: false,
      isFillModeActive: false,
      canUndo: true,
      canRedo: true,
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
      colorSampleFormat: "hex",
    },
  };
}

function createActions() {
  return {
    pointerDown: vi.fn(() => true),
    pointerMove: vi.fn(() => true),
    pointerUp: vi.fn(async () => true),
    resizePointerDown: vi.fn(() => true),
    resizeAnnotationPointerDown: vi.fn(() => true),
    wheel: vi.fn(() => true),
    commitTextDraft: vi.fn(),
    updateTextDraftText: vi.fn(),
    discardTextDraft: vi.fn(),
    selectMoveTool: vi.fn(),
    toggleAnnotationTool: vi.fn(),
    applySelectedAnnotationStyle: vi.fn(),
    updateTextDraftFontSize: vi.fn(),
    commitAnnotationSizeDefault: vi.fn(),
    undoAnnotation: vi.fn(),
    redoAnnotation: vi.fn(),
    cancelSession: vi.fn(async () => undefined),
    completePreviewSelection: vi.fn(async () => undefined),
  } satisfies CaptureWorkspaceViewActions;
}
