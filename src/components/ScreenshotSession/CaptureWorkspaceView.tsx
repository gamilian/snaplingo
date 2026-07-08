import type { PointerEvent, Ref, WheelEvent } from 'react';

import type { AnnotationStyle, AnnotationTool } from './annotationStyle';
import { CaptureEditorToolbar } from './captureEditorToolbar';
import { CaptureMagnifierOverlay } from './captureMagnifierOverlay';
import {
  getCaptureEditorSelectionClassName,
  getCaptureRootClassName,
  getCaptureRootCursorStyle,
  shouldShowCaptureLoadingMask,
} from './capturePresentation';
import {
  CaptureDraftAnnotationOverlay,
  CapturePreviewImage,
  CaptureRenderingOutputBar,
  CaptureSelectedAnnotationBoundsOverlay,
  CaptureSelectionResizeHandles,
  CaptureTextDraftEditor,
  rectStyle,
} from './capturePreviewPresentation';
import { CaptureSelectionOverlayCanvas } from './captureSelectionOverlayRuntime';
import type { CaptureWorkspaceState } from './captureWorkspaceState';
import type { ColorSample, ColorSampleFormat } from './colorSampler';
import type { SelectionHandle } from './selection';
import type { TextAnnotationDraft } from './textAnnotationDraft';
import type {
  AnnotationCommand,
  LogicalRect,
  MonitorSnapshotView,
  Point,
} from './types';

interface CaptureSelectionOverlaySize {
  width: number;
  height: number;
}

interface CaptureWorkspaceViewProps {
  isActive: boolean;
  status: CaptureWorkspaceState['status'];
  viewportBounds: LogicalRect | null;
  error: string | null;
  selection: LogicalRect | null;
  selectionViewportRect: LogicalRect | null;
  previewImageBase64: string | null;
  draftAnnotation: AnnotationCommand | null;
  textDraft: TextAnnotationDraft | null;
  textDraftInputRef: Ref<HTMLTextAreaElement>;
  annotationStyle: AnnotationStyle;
  selectedAnnotationBounds: LogicalRect | null;
  activeAnnotationTool: AnnotationTool | null;
  toolbarPosition: Point | null;
  toolbarWidth: number;
  isAnnotationToolbarVisible: boolean;
  textFontSize: number;
  isTextSizingActive: boolean;
  isFillModeActive: boolean;
  isRenderingOutput: boolean;
  selectionOverlayCanvasRef: Ref<HTMLCanvasElement>;
  selectionOverlayCssSize: CaptureSelectionOverlaySize | null;
  selectionOverlayPixelRatio: number;
  isMagnifierShown: boolean;
  cursorMonitor: MonitorSnapshotView | null;
  cursorViewportPoint: Point | null;
  cursorInMonitorPoint: Point | null;
  magnifierSelection: LogicalRect | null;
  cursorColor: ColorSample | null;
  colorSampleFormat: ColorSampleFormat;
  onRootPointerDown: (event: PointerEvent<HTMLDivElement>) => void;
  onRootPointerMove: (event: PointerEvent<HTMLDivElement>) => void;
  onRootPointerUp: (event: PointerEvent<HTMLDivElement>) => void;
  onRootWheel: (event: WheelEvent<HTMLDivElement>) => void;
  onPreviewPointerDown: (event: PointerEvent<HTMLDivElement>) => void;
  onResizeHandlePointerDown: (
    handle: SelectionHandle,
    event: PointerEvent<HTMLButtonElement>,
  ) => void;
  onCommitTextDraft: () => void;
  onTextDraftTextChange: (text: string) => void;
  onDiscardTextDraft: () => void;
  onSelectMove: () => void;
  onToggleAnnotationTool: (tool: AnnotationTool) => void;
  onApplyAnnotationStyle: (
    nextStyle: AnnotationStyle,
    nextTextFontSize: number,
  ) => void;
  onTextDraftFontSizeChange: (fontSize: number) => void;
  onCancel: () => void | Promise<void>;
  onRunOcr: () => void | Promise<void>;
  onCopy: () => void | Promise<void>;
  onSave: () => void | Promise<void>;
  onQuickSave: () => void | Promise<void>;
}

export function CaptureWorkspaceView({
  activeAnnotationTool,
  annotationStyle,
  colorSampleFormat,
  cursorColor,
  cursorInMonitorPoint,
  cursorMonitor,
  cursorViewportPoint,
  draftAnnotation,
  error,
  isActive,
  isAnnotationToolbarVisible,
  isFillModeActive,
  isMagnifierShown,
  isRenderingOutput,
  isTextSizingActive,
  magnifierSelection,
  onApplyAnnotationStyle,
  onCancel,
  onCommitTextDraft,
  onCopy,
  onDiscardTextDraft,
  onPreviewPointerDown,
  onQuickSave,
  onResizeHandlePointerDown,
  onRootPointerDown,
  onRootPointerMove,
  onRootPointerUp,
  onRootWheel,
  onRunOcr,
  onSave,
  onSelectMove,
  onTextDraftFontSizeChange,
  onTextDraftTextChange,
  onToggleAnnotationTool,
  previewImageBase64,
  selectedAnnotationBounds,
  selection,
  selectionOverlayCanvasRef,
  selectionOverlayCssSize,
  selectionOverlayPixelRatio,
  selectionViewportRect,
  status,
  textDraft,
  textDraftInputRef,
  textFontSize,
  toolbarPosition,
  toolbarWidth,
  viewportBounds,
}: CaptureWorkspaceViewProps) {
  if (!isActive) return null;

  return (
    <div
      className={getCaptureRootClassName(status)}
      style={{
        width: `${viewportBounds?.width ?? window.innerWidth}px`,
        height: `${viewportBounds?.height ?? window.innerHeight}px`,
        cursor: getCaptureRootCursorStyle(status),
      }}
      onPointerDown={onRootPointerDown}
      onPointerMove={onRootPointerMove}
      onPointerUp={onRootPointerUp}
      onWheel={onRootWheel}
      onContextMenu={(event) => event.preventDefault()}
    >
      {shouldShowCaptureLoadingMask(status) && (
        <div className="absolute inset-0 bg-black" aria-label="Loading capture" />
      )}

      {status === 'error' && (
        <div className="absolute left-4 top-4 max-w-md rounded bg-red-950/90 px-3 py-2 text-sm text-red-100 shadow-lg">
          {error}
        </div>
      )}

      {status === 'preview' && selection && selectionViewportRect && (
        <>
          <CapturePreviewImage
            imageBase64={previewImageBase64}
            selectionViewportRect={selectionViewportRect}
          />
          <CaptureDraftAnnotationOverlay
            draftAnnotation={draftAnnotation}
            selectionViewportRect={selectionViewportRect}
          />
          {textDraft && (
            <CaptureTextDraftEditor
              inputRef={textDraftInputRef}
              textDraft={textDraft}
              selectionViewportRect={selectionViewportRect}
              annotationStyle={annotationStyle}
              onCommit={onCommitTextDraft}
              onTextChange={onTextDraftTextChange}
              onDiscard={onDiscardTextDraft}
            />
          )}
          <CaptureSelectedAnnotationBoundsOverlay
            selectedAnnotationBounds={selectedAnnotationBounds}
            selectionViewportRect={selectionViewportRect}
          />
          <div
            className={getCaptureEditorSelectionClassName(
              status,
              Boolean(activeAnnotationTool),
            )}
            style={rectStyle(selectionViewportRect)}
            onPointerDown={onPreviewPointerDown}
          />
          {status === 'preview' && (
            <CaptureSelectionResizeHandles
              selectionViewportRect={selectionViewportRect}
              onResizeHandlePointerDown={onResizeHandlePointerDown}
            />
          )}
          {toolbarPosition && isAnnotationToolbarVisible && (
            <CaptureEditorToolbar
              position={toolbarPosition}
              width={toolbarWidth}
              activeAnnotationTool={activeAnnotationTool}
              annotationStyle={annotationStyle}
              textFontSize={textFontSize}
              textDraftActive={textDraft !== null}
              isTextSizingActive={isTextSizingActive}
              isFillModeActive={isFillModeActive}
              isRenderingOutput={isRenderingOutput}
              onSelectMove={onSelectMove}
              onToggleAnnotationTool={onToggleAnnotationTool}
              onApplyAnnotationStyle={onApplyAnnotationStyle}
              onTextDraftFontSizeChange={onTextDraftFontSizeChange}
              onCancel={onCancel}
              onRunOcr={onRunOcr}
              onCopy={onCopy}
              onSave={onSave}
              onQuickSave={onQuickSave}
            />
          )}
          <CaptureRenderingOutputBar
            isRenderingOutput={isRenderingOutput}
            selectionViewportRect={selectionViewportRect}
          />
        </>
      )}
      <CaptureSelectionOverlayCanvas
        canvasRef={selectionOverlayCanvasRef}
        cssSize={selectionOverlayCssSize}
        pixelRatio={selectionOverlayPixelRatio}
      />
      {isMagnifierShown &&
        cursorMonitor &&
        cursorViewportPoint &&
        cursorInMonitorPoint &&
        viewportBounds && (
          <CaptureMagnifierOverlay
            imageBase64={cursorMonitor.image_base64}
            viewportCursor={cursorViewportPoint}
            imageCursor={cursorInMonitorPoint}
            viewportBounds={viewportBounds}
            imageSize={{
              width: cursorMonitor.logical_bounds.width,
              height: cursorMonitor.logical_bounds.height,
            }}
            selection={magnifierSelection}
            color={cursorColor}
            colorFormat={colorSampleFormat}
          />
        )}
    </div>
  );
}
