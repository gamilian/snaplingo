import { describe, expect, it } from "vitest";
import { readFileSync } from "node:fs";
import {
  getCaptureEditorCommandButtonClassName,
  getCaptureEditorCursorStyle,
  getCaptureEditorIconButtonClassName,
  getCaptureEditorSelectionClassName,
  getCaptureSelectionOverlayCanvasClassName,
  getCaptureEditorToolbarClassName,
  getCaptureRootCursorStyle,
  getCaptureRootClassName,
  shouldShowCaptureLoadingMask,
} from "./capturePresentation";

describe("capture presentation", () => {
  it("keeps the default document canvas transparent before the app mounts", () => {
    const css = readFileSync(
      new URL("../../index.css", import.meta.url),
      "utf8",
    );

    expect(css).toMatch(
      /html,\s*body,\s*#root\s*{[^}]*background:\s*transparent/s,
    );
  });

  it("keeps the capture html shell transparent before React renders", () => {
    const html = readFileSync(
      new URL("../../../index.html", import.meta.url),
      "utf8",
    );

    expect(html).toContain('data-window="capture"');
    expect(html).toMatch(
      /html\[data-window="capture"\][\s\S]*background:\s*transparent/s,
    );
  });

  it("keeps the capture result html shell transparent before React renders", () => {
    const html = readFileSync(
      new URL("../../../index.html", import.meta.url),
      "utf8",
    );

    expect(html).toContain('data-window="capture-result"');
    expect(html).toMatch(
      /html\[data-window="capture-result"\][\s\S]*background:\s*transparent/s,
    );
  });

  it("keeps the capture surface transparent while the snapshot session loads", () => {
    expect(getCaptureRootClassName("loading")).not.toContain("bg-black");
    expect(shouldShowCaptureLoadingMask("loading")).toBe(false);
  });

  it("uses the same transparent capture surface once selection is active", () => {
    expect(getCaptureRootClassName("selecting")).toContain("bg-transparent");
    expect(getCaptureRootClassName("preview")).toContain("bg-transparent");
  });

  it("does not fade the native capture root over underlying windows", () => {
    const className = getCaptureRootClassName("selecting");

    expect(className).not.toContain("opacity-0");
    expect(className).not.toContain("transition-opacity");
  });

  it("hides the system cursor while the canvas crosshair is active", () => {
    expect(getCaptureRootCursorStyle("selecting")).toBe("none");
    expect(getCaptureRootCursorStyle("preview")).toBe("default");
    expect(getCaptureRootCursorStyle("preview", "text")).toBe("text");
    expect(getCaptureRootCursorStyle("preview", "mosaic")).toContain(
      "data:image/svg+xml",
    );
    expect(getCaptureEditorCursorStyle("pen")).toContain("data:image/svg+xml");
    expect(getCaptureEditorCursorStyle("eraser")).toContain(
      "data:image/svg+xml",
    );
  });

  it("keeps the selection overlay canvas visible on the first revealed frame", () => {
    const className = getCaptureSelectionOverlayCanvasClassName();

    expect(className).not.toContain("opacity-0");
    expect(className).not.toContain("transition-opacity");
  });

  it("uses a light floating toolbar for the editing surface", () => {
    const className = getCaptureEditorToolbarClassName();

    expect(className).toContain("h-[42px]");
    expect(className).toContain("gap-1");
    expect(className).toContain("rounded-[12px]");
    expect(className).toContain("bg-white/95");
    expect(className).toContain("text-slate-600");
  });

  it("uses compact editor controls for small capture selections", () => {
    expect(getCaptureEditorIconButtonClassName()).toContain("h-7 w-7");
    expect(getCaptureEditorCommandButtonClassName()).toContain("h-7");
    expect(getCaptureEditorCommandButtonClassName()).toContain("text-[11px]");
    expect(getCaptureEditorCommandButtonClassName("primary")).toContain("w-7");
  });

  it("uses short text for escape and OCR while keeping output actions as SVG icons", () => {
    const toolbar = readFileSync(
      new URL("./captureEditorToolbar.tsx", import.meta.url),
      "utf8",
    );

    expect(toolbar).toMatch(/>\s*ESC\s*</);
    expect(toolbar).toMatch(/>\s*OCR\s*</);
    expect(toolbar).not.toContain("取消 (Esc)");
    expect(toolbar).not.toContain("完成 (Enter)");
    expect(toolbar).not.toContain("<CheckIcon />");
    expect(toolbar).toContain("<SaveIcon />");
    expect(toolbar).toContain("<CopyIcon />");
    expect(toolbar).toContain("<UndoIcon />");
    expect(toolbar).toContain("<RedoIcon />");
    expect(toolbar).toContain("<PinIcon />");
    expect(toolbar).toMatch(
      /<button(?:(?!<\/button>)[\s\S])*className=\{getCaptureEditorCommandButtonClassName\(\)\}(?:(?!<\/button>)[\s\S])*title="OCR"/,
    );
    expect(toolbar).toContain('title="Copy (Enter)"');
  });

  it("shows the text draft border only while it is hovered or focused", () => {
    const preview = readFileSync(
      new URL("./capturePreviewPresentation.tsx", import.meta.url),
      "utf8",
    );

    expect(preview).toContain("border-transparent bg-transparent");
    expect(preview).toContain("hover:border-white/70 focus:border-white/70");
    expect(preview).not.toContain("bg-black/15");
  });

  it("throttles annotation canvas painting and reuses its mosaic buffer", () => {
    const canvas = readFileSync(
      new URL("./captureAnnotationCanvas.tsx", import.meta.url),
      "utf8",
    );

    expect(canvas).toContain("requestAnimationFrame");
    expect(canvas).toContain("cancelAnimationFrame");
    expect(canvas).toContain("if (canvas.width !== pixelWidth)");
    expect(canvas).toContain("mosaicBufferRef.current");
  });

  it("keeps toolbar placement dimensions synchronized with the compact surface", () => {
    const runtimeView = readFileSync(
      new URL("./useCaptureWorkspaceRuntimeView.ts", import.meta.url),
      "utf8",
    );

    expect(runtimeView).toContain(
      "const TOOLBAR_SIZE = { width: 700, height: 42 };",
    );
  });

  it("keeps the selection hit layer transparent and primary editor affordances", () => {
    expect(getCaptureEditorSelectionClassName("preview")).toEqual(
      expect.stringContaining("bg-transparent"),
    );
    expect(getCaptureEditorSelectionClassName("preview")).not.toContain(
      "border-[#5b7fff]",
    );
    expect(getCaptureEditorSelectionClassName("preview")).toEqual(
      expect.stringContaining("cursor-default"),
    );
    expect(getCaptureEditorSelectionClassName("preview")).not.toContain(
      "cursor-move",
    );
    expect(getCaptureEditorSelectionClassName("preview")).toEqual(
      expect.stringContaining("rounded-[8px]"),
    );
    expect(getCaptureEditorIconButtonClassName(true)).toEqual(
      expect.stringContaining("border-[#5b7fff]"),
    );
    expect(getCaptureEditorCommandButtonClassName("primary")).toEqual(
      expect.stringContaining("bg-[#5b7fff]"),
    );
  });
});
