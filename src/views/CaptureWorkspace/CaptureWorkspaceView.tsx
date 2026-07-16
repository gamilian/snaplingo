import type { PointerEvent, ReactNode, Ref, WheelEvent } from 'react';

import type {
  CaptureWorkspacePointerInput,
  CaptureWorkspaceRuntimeActions,
} from './captureWorkspaceRuntimeTypes';
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
import type { CaptureCandidateDetectionMode } from './captureWorkspaceState';
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
  readonly candidateDetectionMode: CaptureCandidateDetectionMode;
  readonly error: string | null;
  readonly viewportBounds: LogicalRect | null;
  readonly selectionBounds: LogicalRect | null;
  readonly isRenderingOutput: boolean;
  readonly silentOcrHint: {
    readonly status: 'loading' | 'success';
    readonly point: Point;
  } | null;
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
    readonly cursorScreenPoint: Point | null;
    readonly cursorInMonitorPoint: Point | null;
    readonly cursorColor: ColorSample | null;
    readonly colorSampleFormat: ColorSampleFormat;
    readonly sourceImage: HTMLImageElement | null;
    readonly zoom: number;
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
  | 'commitAnnotationSizeDefault'
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

      {renderState.silentOcrHint && (
        <div
          role="status"
          aria-label={
            renderState.silentOcrHint.status === 'loading'
              ? 'OCR 识别中'
              : 'OCR 已复制'
          }
          className="pointer-events-none absolute z-[80] flex items-center gap-2 rounded-lg bg-slate-950/90 px-3 py-2 text-xs font-medium text-white shadow-xl"
          style={{
            left: renderState.silentOcrHint.point.x + 12,
            top: renderState.silentOcrHint.point.y + 12,
          }}
        >
          <span
            className={
              renderState.silentOcrHint.status === 'loading'
                ? 'h-3.5 w-3.5 animate-spin rounded-full border-2 border-white/40 border-t-white'
                : 'grid h-3.5 w-3.5 place-items-center rounded-full bg-emerald-500 text-[10px]'
            }
          >
            {renderState.silentOcrHint.status === 'success' ? '✓' : ''}
          </span>
          {renderState.silentOcrHint.status === 'loading'
            ? '正在识别…'
            : '已复制到剪贴板'}
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
                onCommitSizeDefault={actions.commitAnnotationSizeDefault}
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
        renderState.magnifier.cursorScreenPoint &&
        renderState.magnifier.cursorInMonitorPoint &&
        renderState.magnifier.sourceImage &&
        renderState.viewportBounds && (
          <CaptureMagnifierOverlay
            image={renderState.magnifier.sourceImage}
            viewportCursor={renderState.magnifier.cursorViewportPoint}
            screenCursor={renderState.magnifier.cursorScreenPoint}
            imageCursor={renderState.magnifier.cursorInMonitorPoint}
            viewportBounds={renderState.viewportBounds}
            imageSize={{
              width: renderState.magnifier.cursorMonitor.logical_bounds.width,
              height: renderState.magnifier.cursorMonitor.logical_bounds.height,
            }}
            color={renderState.magnifier.cursorColor}
            colorFormat={renderState.magnifier.colorSampleFormat}
            zoom={renderState.magnifier.zoom}
          />
        )}
      {renderState.status === 'selecting' && (
        <CaptureSelectionShortcutHints
          detectionMode={renderState.candidateDetectionMode}
        />
      )}
    </div>
  );
}

function CaptureSelectionShortcutHints({
  detectionMode,
}: {
  detectionMode: CaptureCandidateDetectionMode;
}) {
  const shortcutKeys = getCaptureShortcutKeyLabels();
  return (
    <aside
      aria-label="选区快捷键提示"
      className="pointer-events-none absolute bottom-5 left-5 w-[330px] overflow-hidden rounded-lg border border-white/25 bg-slate-900/[0.55] text-[11px] text-white/90 shadow-lg backdrop-blur-lg"
    >
      <HintRow keys={['W', 'A', 'S', 'D']}>
        将鼠标指针移动 1 像素
      </HintRow>
      <HintRow keys={['Tab']}>
        切换检测窗口 / 界面元素
        <span className="ml-2 text-blue-200/90">
          当前：{detectionMode === 'window' ? '窗口' : '界面元素'}
        </span>
      </HintRow>
      <HintRow keys={[shortcutKeys.primary, 'A']}>
        设置截屏区域为当前屏幕
      </HintRow>
      <HintRow keys={[shortcutKeys.shift, shortcutKeys.primary, 'A']}>
        设置截屏区域为全屏
      </HintRow>
    </aside>
  );
}

export function getCaptureShortcutKeyLabels(
  platform = typeof navigator === 'undefined' ? '' : navigator.platform,
) {
  const isApplePlatform = /Mac|iPhone|iPad|iPod/i.test(platform);
  return isApplePlatform
    ? { primary: '⌘', shift: '⇧' }
    : { primary: 'Ctrl', shift: 'Shift' };
}

function HintRow({
  keys,
  children,
}: {
  keys: string[];
  children: ReactNode;
}) {
  return (
    <div className="flex min-h-9 items-center gap-3 border-b border-white/[0.08] px-3 last:border-b-0">
      <span className="flex min-w-[84px] items-center gap-1">
        {keys.map((key, index) => (
          <kbd
            key={`${key}-${index}`}
            className="grid min-w-5 place-items-center rounded border border-white/30 bg-white/[0.08] px-1 py-0.5 font-mono text-[10px] font-semibold"
          >
            {key}
          </kbd>
        ))}
      </span>
      <span className="leading-4 text-white/[0.85]">{children}</span>
    </div>
  );
}
