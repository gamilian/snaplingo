import type { PointerEvent, Ref, WheelEvent } from 'react';

import type {
  CaptureWorkspacePointerInput,
  CaptureWorkspaceRenderState,
  CaptureWorkspaceRuntimeActions,
} from '../../application/capture-workspace/types';
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
import type { CaptureWorkspaceDerivedState } from './captureWorkspaceDerived';
import { getCaptureWorkspacePointerPoint } from './captureWorkspacePointer';
import type { SelectionHandle } from './selection';
import type { LogicalRect } from './types';

interface CaptureSelectionOverlaySize {
  width: number;
  height: number;
}

export interface CaptureWorkspaceViewRenderState
  extends CaptureWorkspaceRenderState,
    Omit<CaptureWorkspaceDerivedState, 'hasHydratedPixelSource'> {
  readonly toolbarWidth: number;
  readonly magnifierSelection: LogicalRect | null;
  readonly textDraftInputRef: Ref<HTMLTextAreaElement>;
  readonly selectionOverlayCanvasRef: Ref<HTMLCanvasElement>;
  readonly selectionOverlayCssSize: CaptureSelectionOverlaySize | null;
  readonly selectionOverlayPixelRatio: number;
}

interface CaptureWorkspaceViewProps {
  renderState: CaptureWorkspaceViewRenderState;
  actions: CaptureWorkspaceRuntimeActions;
}

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
}: CaptureWorkspaceViewProps) {
  if (renderState.status === 'idle') return null;

  const completeSelection = (
    action: Parameters<CaptureWorkspaceRuntimeActions['completePreviewSelection']>[0],
  ) => {
    if (!renderState.selection) return;
    return actions.completePreviewSelection(action, renderState.selection);
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
        metaKey: event.metaKey,
        ctrlKey: event.ctrlKey,
        altKey: event.altKey,
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
        cursor: getCaptureRootCursorStyle(renderState.status),
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

      {renderState.status === 'error' && (
        <div className="absolute left-4 top-4 max-w-md rounded bg-red-950/90 px-3 py-2 text-sm text-red-100 shadow-lg">
          {renderState.error}
        </div>
      )}

      {renderState.status === 'preview' &&
        renderState.selection &&
        renderState.selectionViewportRect && (
          <>
            <CapturePreviewImage
              imageBase64={renderState.previewImageBase64}
              selectionViewportRect={renderState.selectionViewportRect}
            />
            <CaptureDraftAnnotationOverlay
              draftAnnotation={renderState.draftAnnotation}
              selectionViewportRect={renderState.selectionViewportRect}
            />
            {renderState.textDraft && (
              <CaptureTextDraftEditor
                inputRef={renderState.textDraftInputRef}
                textDraft={renderState.textDraft}
                selectionViewportRect={renderState.selectionViewportRect}
                annotationStyle={renderState.annotationStyle}
                onCommit={actions.commitTextDraft}
                onTextChange={actions.updateTextDraftText}
                onDiscard={actions.discardTextDraft}
              />
            )}
            <CaptureSelectedAnnotationBoundsOverlay
              selectedAnnotationBounds={renderState.selectedAnnotationBounds}
              selectionViewportRect={renderState.selectionViewportRect}
            />
            <div
              className={getCaptureEditorSelectionClassName(
                renderState.status,
                Boolean(renderState.activeAnnotationTool),
              )}
              style={rectStyle(renderState.selectionViewportRect)}
              onPointerDown={(event) => {
                dispatchCaptureWorkspacePreviewPointerDown({
                  event,
                  selectionBounds: renderState.selectionBounds,
                  pointerDown: actions.pointerDown,
                });
              }}
            />
            <CaptureSelectionResizeHandles
              selectionViewportRect={renderState.selectionViewportRect}
              onResizeHandlePointerDown={(handle, event) => {
                dispatchCaptureWorkspaceResizePointerDown({
                  handle,
                  event,
                  selectionBounds: renderState.selectionBounds,
                  resizePointerDown: actions.resizePointerDown,
                });
              }}
            />
            {renderState.toolbarPosition &&
              renderState.isAnnotationToolbarVisible && (
                <CaptureEditorToolbar
                  position={renderState.toolbarPosition}
                  width={renderState.toolbarWidth}
                  activeAnnotationTool={renderState.activeAnnotationTool}
                  annotationStyle={renderState.annotationStyle}
                  textFontSize={renderState.textFontSize}
                  textDraftActive={renderState.textDraft !== null}
                  isTextSizingActive={renderState.isTextSizingActive}
                  isFillModeActive={renderState.isFillModeActive}
                  isRenderingOutput={renderState.isRenderingOutput}
                  onSelectMove={actions.selectMoveTool}
                  onToggleAnnotationTool={actions.toggleAnnotationTool}
                  onApplyAnnotationStyle={actions.applySelectedAnnotationStyle}
                  onTextDraftFontSizeChange={actions.updateTextDraftFontSize}
                  onCancel={actions.cancelSession}
                  onRunOcr={() => completeSelection('ocr')}
                  onCopy={() => completeSelection('copy')}
                  onSave={() => completeSelection('save')}
                  onQuickSave={() => completeSelection('quick-save')}
                />
              )}
            <CaptureRenderingOutputBar
              isRenderingOutput={renderState.isRenderingOutput}
              selectionViewportRect={renderState.selectionViewportRect}
            />
          </>
        )}
      <CaptureSelectionOverlayCanvas
        canvasRef={renderState.selectionOverlayCanvasRef}
        cssSize={renderState.selectionOverlayCssSize}
        pixelRatio={renderState.selectionOverlayPixelRatio}
      />
      {renderState.isMagnifierShown &&
        renderState.cursorMonitor &&
        renderState.cursorViewportPoint &&
        renderState.cursorInMonitorPoint &&
        renderState.viewportBounds && (
          <CaptureMagnifierOverlay
            imageBase64={renderState.cursorMonitor.image_base64}
            viewportCursor={renderState.cursorViewportPoint}
            imageCursor={renderState.cursorInMonitorPoint}
            viewportBounds={renderState.viewportBounds}
            imageSize={{
              width: renderState.cursorMonitor.logical_bounds.width,
              height: renderState.cursorMonitor.logical_bounds.height,
            }}
            selection={renderState.magnifierSelection}
            color={renderState.cursorColor}
            colorFormat={renderState.colorSampleFormat}
          />
        )}
    </div>
  );
}
