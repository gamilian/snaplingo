import type { PointerEvent, Ref, WheelEvent } from 'react';

import type {
  CaptureWorkspacePointerInput,
  CaptureWorkspaceRuntimeActions,
} from '../../application/capture-workspace/types';
import {
  ANNOTATION_COLORS,
  type AnnotationColor,
  type AnnotationStyle,
  type AnnotationTool,
} from './annotationStyle';
import { CaptureAnnotationCanvas } from './captureAnnotationCanvas';
import { CaptureEditorToolbar } from './captureEditorToolbar';
import { CaptureMagnifierOverlay } from './captureMagnifierOverlay';
import {
  getCaptureEditorSelectionClassName,
  getCaptureRootClassName,
  getCaptureRootCursorStyle,
  shouldShowCaptureLoadingMask,
} from './capturePresentation';
import {
  CapturePreviewImage,
  CaptureRenderingOutputBar,
  CaptureSelectedAnnotationBoundsOverlay,
  CaptureSelectionResizeHandles,
  CaptureTextDraftEditor,
  rectStyle,
} from './capturePreviewPresentation';
import { CaptureSelectionOverlayCanvas } from './captureSelectionOverlayRuntime';
import { getCaptureWorkspacePointerPoint } from './captureWorkspacePointer';
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

export interface CaptureWorkspaceViewRenderState {
  readonly status: 'idle' | 'loading' | 'selecting' | 'preview' | 'error';
  readonly error: string | null;
  readonly viewportBounds: LogicalRect | null;
  readonly selectionBounds: LogicalRect | null;
  readonly isRenderingOutput: boolean;
  readonly editor: {
    readonly selection: LogicalRect | null;
    readonly selectionViewportRect: LogicalRect | null;
    readonly previewImageBase64: string | null;
    readonly annotations: AnnotationCommand[];
    readonly draftAnnotation: AnnotationCommand | null;
    readonly textDraft: TextAnnotationDraft | null;
    readonly annotationStyle: AnnotationStyle;
    readonly selectedAnnotationBounds: LogicalRect | null;
    readonly activeAnnotationTool: AnnotationTool | null;
  };
  readonly toolbar: {
    readonly position: Point | null;
    readonly width: number;
    readonly isVisible: boolean;
    readonly textFontSize: number;
    readonly isTextSizingActive: boolean;
    readonly isFillModeActive: boolean;
    readonly canUndo: boolean;
    readonly canRedo: boolean;
  };
  readonly dom: {
    readonly textDraftInputRef: Ref<HTMLTextAreaElement>;
    readonly selectionOverlay: {
      readonly canvasRef: Ref<HTMLCanvasElement>;
      readonly cssSize: CaptureSelectionOverlaySize | null;
      readonly pixelRatio: number;
    };
  };
  readonly magnifier: {
    readonly isShown: boolean;
    readonly cursorMonitor: MonitorSnapshotView | null;
    readonly cursorViewportPoint: Point | null;
    readonly cursorInMonitorPoint: Point | null;
    readonly selection: LogicalRect | null;
    readonly cursorColor: ColorSample | null;
    readonly colorSampleFormat: ColorSampleFormat;
  };
}

export type CaptureWorkspaceViewActions = Pick<
  CaptureWorkspaceRuntimeActions,
  | 'pointerDown'
  | 'pointerMove'
  | 'pointerUp'
  | 'resizePointerDown'
  | 'resizeAnnotationPointerDown'
  | 'wheel'
  | 'commitTextDraft'
  | 'updateTextDraftText'
  | 'discardTextDraft'
  | 'selectMoveTool'
  | 'toggleAnnotationTool'
  | 'applySelectedAnnotationStyle'
  | 'updateTextDraftFontSize'
  | 'undoAnnotation'
  | 'redoAnnotation'
  | 'cancelSession'
  | 'completePreviewSelection'
>;

interface CaptureWorkspaceViewProps {
  renderState: CaptureWorkspaceViewRenderState;
  actions: CaptureWorkspaceViewActions;
  annotationColorPresets?: readonly AnnotationColor[];
  onUpdateAnnotationColorPresets?: (
    colors: AnnotationColor[],
  ) => void | Promise<unknown>;
}

function ignoreAnnotationColorPresetUpdate() {}

interface CaptureWorkspaceDomPointerEvent {
  clientX: number;
  clientY: number;
  pointerId: number;
  button: number;
  detail: number;
  metaKey: boolean;
  ctrlKey: boolean;
  altKey: boolean;
  shiftKey: boolean;
  currentTarget: {
    setPointerCapture(pointerId: number): void;
  };
  preventDefault(): void;
  stopPropagation(): void;
}

export function dispatchCaptureWorkspacePreviewPointerDown({
  event,
  selectionBounds,
  pointerDown,
}: {
  event: CaptureWorkspaceDomPointerEvent;
  selectionBounds: LogicalRect | null;
  pointerDown(input: CaptureWorkspacePointerInput): boolean;
}) {
  if (!selectionBounds) return false;

  event.currentTarget.setPointerCapture(event.pointerId);
  const handled = pointerDown({
    point: getCaptureWorkspacePointerPoint(event, selectionBounds),
    button: event.button,
    detail: event.detail,
    metaKey: event.metaKey,
    ctrlKey: event.ctrlKey,
    altKey: event.altKey,
    shiftKey: event.shiftKey,
    source: 'preview',
  });
  if (handled) {
    event.preventDefault();
    event.stopPropagation();
  }
  return handled;
}

export function dispatchCaptureWorkspaceResizePointerDown({
  handle,
  event,
  selectionBounds,
  resizePointerDown,
}: {
  handle: SelectionHandle;
  event: CaptureWorkspaceDomPointerEvent;
  selectionBounds: LogicalRect | null;
  resizePointerDown(
    handle: SelectionHandle,
    input: CaptureWorkspacePointerInput,
  ): boolean;
}) {
  if (!selectionBounds) return false;

  event.currentTarget.setPointerCapture(event.pointerId);
  const handled = resizePointerDown(handle, {
    point: getCaptureWorkspacePointerPoint(event, selectionBounds),
    button: event.button,
    shiftKey: event.shiftKey,
    source: 'preview',
  });
  if (handled) {
    event.preventDefault();
    event.stopPropagation();
  }
  return handled;
}

export function CaptureWorkspaceView({
  renderState,
  actions,
  annotationColorPresets = ANNOTATION_COLORS,
  onUpdateAnnotationColorPresets = ignoreAnnotationColorPresetUpdate,
}: CaptureWorkspaceViewProps) {
  if (renderState.status === 'idle') return null;

  const completeSelection = (
    action: Parameters<CaptureWorkspaceViewActions['completePreviewSelection']>[0],
  ) => {
    if (!renderState.editor.selection) return;
    return actions.completePreviewSelection(
      action,
      renderState.editor.selection,
    );
  };
  const handleRootPointerDown = (event: PointerEvent<HTMLDivElement>) => {
    if (!renderState.selectionBounds) return;

    event.currentTarget.setPointerCapture(event.pointerId);
    const handled = actions.pointerDown({
      point: getCaptureWorkspacePointerPoint(
        event,
        renderState.selectionBounds,
      ),
      button: event.button,
      shiftKey: event.shiftKey,
      source: 'root',
    });
    if (handled) event.preventDefault();
  };
  const handleRootPointerMove = (event: PointerEvent<HTMLDivElement>) => {
    if (!renderState.selectionBounds) return;
    actions.pointerMove({
      point: getCaptureWorkspacePointerPoint(event, renderState.selectionBounds),
      button: event.button,
      shiftKey: event.shiftKey,
      source: 'root',
    });
  };
  const handleRootPointerUp = (event: PointerEvent<HTMLDivElement>) => {
    if (!renderState.selectionBounds) return;
    void actions.pointerUp({
      point: getCaptureWorkspacePointerPoint(event, renderState.selectionBounds),
      button: event.button,
      shiftKey: event.shiftKey,
      source: 'root',
    });
  };
  const handleRootWheel = (event: WheelEvent<HTMLDivElement>) => {
    if (
      actions.wheel({
        deltaY: event.deltaY,
        metaKey: event.metaKey ?? false,
        ctrlKey: event.ctrlKey ?? false,
        altKey: event.altKey ?? false,
      })
    ) {
      event.preventDefault();
    }
  };

  return (
    <div
      className={getCaptureRootClassName(renderState.status)}
      style={{
        width: `${renderState.viewportBounds?.width ?? window.innerWidth}px`,
        height: `${renderState.viewportBounds?.height ?? window.innerHeight}px`,
        cursor: getCaptureRootCursorStyle(
          renderState.status,
          renderState.editor.activeAnnotationTool,
          renderState.editor.annotationStyle.strokeWidth,
        ),
      }}
      onPointerDown={handleRootPointerDown}
      onPointerMove={handleRootPointerMove}
      onPointerUp={handleRootPointerUp}
      onWheel={handleRootWheel}
      onContextMenu={(event) => event.preventDefault()}
    >
      {shouldShowCaptureLoadingMask(renderState.status) && (
        <div className="absolute inset-0 bg-black" aria-label="Loading capture" />
      )}

      {renderState.error && (
        <div className="absolute left-4 top-4 max-w-md rounded bg-red-950/90 px-3 py-2 text-sm text-red-100 shadow-lg">
          {renderState.error}
        </div>
      )}

      {renderState.status === 'preview' &&
        renderState.editor.selection &&
        renderState.editor.selectionViewportRect && (
          <>
            <CapturePreviewImage
              imageBase64={renderState.editor.previewImageBase64}
              selectionViewportRect={renderState.editor.selectionViewportRect}
            />
            <CaptureAnnotationCanvas
              imageBase64={renderState.editor.previewImageBase64}
              annotations={renderState.editor.annotations}
              draftAnnotation={renderState.editor.draftAnnotation}
              selectionViewportRect={renderState.editor.selectionViewportRect}
            />
            {renderState.editor.textDraft && (
              <CaptureTextDraftEditor
                inputRef={renderState.dom.textDraftInputRef}
                textDraft={renderState.editor.textDraft}
                selectionViewportRect={renderState.editor.selectionViewportRect}
                annotationStyle={renderState.editor.annotationStyle}
                onCommit={actions.commitTextDraft}
                onTextChange={actions.updateTextDraftText}
                onDiscard={actions.discardTextDraft}
              />
            )}
            <CaptureSelectedAnnotationBoundsOverlay
              selectedAnnotationBounds={
                renderState.editor.selectedAnnotationBounds
              }
              selectionViewportRect={renderState.editor.selectionViewportRect}
              onResizeHandlePointerDown={(handle, event) => {
                dispatchCaptureWorkspaceResizePointerDown({
                  handle,
                  event,
                  selectionBounds: renderState.selectionBounds,
                  resizePointerDown: actions.resizeAnnotationPointerDown,
                });
              }}
            />
            <div
              className={getCaptureEditorSelectionClassName(
                renderState.status,
                renderState.editor.activeAnnotationTool,
              )}
              style={{
                ...rectStyle(renderState.editor.selectionViewportRect),
                cursor: getCaptureRootCursorStyle(
                  renderState.status,
                  renderState.editor.activeAnnotationTool,
                  renderState.editor.annotationStyle.strokeWidth,
                ),
              }}
              onPointerDown={(event) => {
                dispatchCaptureWorkspacePreviewPointerDown({
                  event,
                  selectionBounds: renderState.selectionBounds,
                  pointerDown: actions.pointerDown,
                });
              }}
            />
            <CaptureSelectionResizeHandles
              selectionViewportRect={renderState.editor.selectionViewportRect}
              onResizeHandlePointerDown={(handle, event) => {
                dispatchCaptureWorkspaceResizePointerDown({
                  handle,
                  event,
                  selectionBounds: renderState.selectionBounds,
                  resizePointerDown: actions.resizePointerDown,
                });
              }}
            />
            {renderState.toolbar.position && renderState.toolbar.isVisible && (
              <CaptureEditorToolbar
                position={renderState.toolbar.position}
                width={renderState.toolbar.width}
                activeAnnotationTool={renderState.editor.activeAnnotationTool}
                annotationStyle={renderState.editor.annotationStyle}
                annotationColorPresets={annotationColorPresets}
                textFontSize={renderState.toolbar.textFontSize}
                textDraftActive={renderState.editor.textDraft !== null}
                isTextSizingActive={renderState.toolbar.isTextSizingActive}
                isFillModeActive={renderState.toolbar.isFillModeActive}
                canUndo={renderState.toolbar.canUndo}
                canRedo={renderState.toolbar.canRedo}
                isRenderingOutput={renderState.isRenderingOutput}
                onSelectMove={actions.selectMoveTool}
                onToggleAnnotationTool={actions.toggleAnnotationTool}
                onApplyAnnotationStyle={actions.applySelectedAnnotationStyle}
                onUpdateAnnotationColorPresets={
                  onUpdateAnnotationColorPresets
                }
                onTextDraftFontSizeChange={actions.updateTextDraftFontSize}
                onUndo={actions.undoAnnotation}
                onRedo={actions.redoAnnotation}
                onCancel={actions.cancelSession}
                onRunOcr={() => completeSelection('ocr')}
                onCopy={() => completeSelection('copy')}
                onSave={() => completeSelection('save')}
                onQuickSave={() => completeSelection('quick-save')}
                onPin={() => completeSelection('pin')}
                onFavorite={() => completeSelection('favorite')}
              />
            )}
            <CaptureRenderingOutputBar
              isRenderingOutput={renderState.isRenderingOutput}
              selectionViewportRect={renderState.editor.selectionViewportRect}
            />
          </>
        )}
      <CaptureSelectionOverlayCanvas
        canvasRef={renderState.dom.selectionOverlay.canvasRef}
        cssSize={renderState.dom.selectionOverlay.cssSize}
        pixelRatio={renderState.dom.selectionOverlay.pixelRatio}
      />
      {renderState.magnifier.isShown &&
        renderState.magnifier.cursorMonitor &&
        renderState.magnifier.cursorViewportPoint &&
        renderState.magnifier.cursorInMonitorPoint &&
        renderState.viewportBounds && (
          <CaptureMagnifierOverlay
            imageBase64={renderState.magnifier.cursorMonitor.image_base64}
            viewportCursor={renderState.magnifier.cursorViewportPoint}
            imageCursor={renderState.magnifier.cursorInMonitorPoint}
            viewportBounds={renderState.viewportBounds}
            imageSize={{
              width: renderState.magnifier.cursorMonitor.logical_bounds.width,
              height: renderState.magnifier.cursorMonitor.logical_bounds.height,
            }}
            selection={renderState.magnifier.selection}
            color={renderState.magnifier.cursorColor}
            colorFormat={renderState.magnifier.colorSampleFormat}
          />
        )}
    </div>
  );
}
