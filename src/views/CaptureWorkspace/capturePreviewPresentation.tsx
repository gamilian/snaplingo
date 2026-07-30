import type { PointerEvent, Ref } from 'react';
import {
  annotationColorToCss,
  type AnnotationStyle,
} from './annotationStyle';
import type { SelectionHandle } from './selection';
import type { TextAnnotationDraft } from './textAnnotationDraft';
import type { LogicalRect } from './types';

const SELECTION_HANDLES: SelectionHandle[] = ['nw', 'n', 'ne', 'e', 'se', 's', 'sw', 'w'];

const resizeHandleClassNames: Record<SelectionHandle, string> = {
  nw: 'left-0 top-0 -translate-x-1/2 -translate-y-1/2 cursor-nwse-resize',
  n: 'left-1/2 top-0 -translate-x-1/2 -translate-y-1/2 cursor-ns-resize',
  ne: 'right-0 top-0 translate-x-1/2 -translate-y-1/2 cursor-nesw-resize',
  e: 'right-0 top-1/2 translate-x-1/2 -translate-y-1/2 cursor-ew-resize',
  se: 'bottom-0 right-0 translate-x-1/2 translate-y-1/2 cursor-nwse-resize',
  s: 'bottom-0 left-1/2 -translate-x-1/2 translate-y-1/2 cursor-ns-resize',
  sw: 'bottom-0 left-0 -translate-x-1/2 translate-y-1/2 cursor-nesw-resize',
  w: 'left-0 top-1/2 -translate-x-1/2 -translate-y-1/2 cursor-ew-resize',
};

const resizeEdgeClassNames: Record<'n' | 'e' | 's' | 'w', string> = {
  n: 'left-2 right-2 -top-1.5 h-3 cursor-ns-resize',
  e: '-right-1.5 top-2 bottom-2 w-3 cursor-ew-resize',
  s: 'left-2 right-2 -bottom-1.5 h-3 cursor-ns-resize',
  w: '-left-1.5 top-2 bottom-2 w-3 cursor-ew-resize',
};

export function rectStyle(rect: LogicalRect) {
  return {
    left: `${rect.x}px`,
    top: `${rect.y}px`,
    width: `${rect.width}px`,
    height: `${rect.height}px`,
  };
}

export function annotationRectToViewportRect(
  rect: LogicalRect,
  selectionViewportRect: LogicalRect,
): LogicalRect {
  return {
    x: selectionViewportRect.x + rect.x,
    y: selectionViewportRect.y + rect.y,
    width: rect.width,
    height: rect.height,
  };
}

interface CapturePreviewImageProps {
  imageBase64: string | null;
  selectionViewportRect: LogicalRect;
}

export function CapturePreviewImage({
  imageBase64,
  selectionViewportRect,
}: CapturePreviewImageProps) {
  if (!imageBase64) return null;

  return (
    <img
      src={`data:image/png;base64,${imageBase64}`}
      className="absolute object-fill"
      style={rectStyle(selectionViewportRect)}
      draggable={false}
    />
  );
}

interface CaptureSelectedAnnotationBoundsOverlayProps {
  selectedAnnotationBounds: LogicalRect | null;
  selectionViewportRect: LogicalRect;
  onResizeHandlePointerDown: (
    handle: SelectionHandle,
    event: PointerEvent<HTMLButtonElement>,
  ) => void;
}

export function CaptureSelectedAnnotationBoundsOverlay({
  selectedAnnotationBounds,
  selectionViewportRect,
  onResizeHandlePointerDown,
}: CaptureSelectedAnnotationBoundsOverlayProps) {
  if (!selectedAnnotationBounds) return null;

  return (
    <div
      className="pointer-events-none absolute border border-dashed border-white/75 shadow-[0_0_0_1px_rgba(0,0,0,0.45)]"
      style={{
        ...rectStyle(
          annotationRectToViewportRect(
            selectedAnnotationBounds,
            selectionViewportRect,
          ),
        ),
        zIndex: 4,
      }}
    >
      {(['n', 'e', 's', 'w'] as const).map((handle) => (
        <button
          key={`edge-${handle}`}
          type="button"
          className={`pointer-events-auto absolute border-0 bg-transparent p-0 ${resizeEdgeClassNames[handle]}`}
          aria-label={`Resize annotation edge ${handle}`}
          onPointerDown={(event) => onResizeHandlePointerDown(handle, event)}
        />
      ))}
      {SELECTION_HANDLES.map((handle) => (
        <button
          key={handle}
          type="button"
          className={`pointer-events-auto absolute z-[3] h-1.5 w-1.5 rounded-full border border-white/80 bg-slate-700/55 p-0 shadow-[0_0_0_1px_rgba(0,0,0,0.3)] ${resizeHandleClassNames[handle]}`}
          aria-label={`Resize annotation ${handle}`}
          onPointerDown={(event) => onResizeHandlePointerDown(handle, event)}
        />
      ))}
    </div>
  );
}

interface CaptureTextDraftEditorProps {
  inputRef: Ref<HTMLTextAreaElement>;
  textDraft: TextAnnotationDraft;
  selectionViewportRect: LogicalRect;
  annotationStyle: AnnotationStyle;
  onCommit: () => void;
  onTextChange: (text: string) => void;
  onDiscard: () => void;
}

export function CaptureTextDraftEditor({
  inputRef,
  textDraft,
  selectionViewportRect,
  annotationStyle,
  onCommit,
  onTextChange,
  onDiscard,
}: CaptureTextDraftEditorProps) {
  const lines = textDraft.text.split('\n');
  const longestLine = Math.max(...lines.map((line) => line.length), 1);
  const inputWidth = Math.min(
    Math.max(40, Math.ceil(longestLine * textDraft.fontSize * 0.62 + 12)),
    Math.max(40, selectionViewportRect.width - textDraft.position.x),
  );

  return (
    <textarea
      ref={inputRef}
      data-screenshot-text-draft="true"
      className="absolute resize-none overflow-hidden border border-transparent bg-transparent px-1 py-0 text-left outline-none ring-0 hover:border-white/70 focus:border-white/70 focus:shadow-[0_0_0_1px_rgba(0,0,0,0.45)]"
      style={{
        left: `${selectionViewportRect.x + textDraft.position.x}px`,
        top: `${
          selectionViewportRect.y +
          textDraft.position.y -
          textDraft.fontSize
        }px`,
        width: `${inputWidth}px`,
        minHeight: `${Math.ceil(textDraft.fontSize * 1.35 * lines.length)}px`,
        color: annotationColorToCss(annotationStyle.color),
        fontSize: `${textDraft.fontSize}px`,
        lineHeight: 1.2,
        zIndex: 2,
      }}
      value={textDraft.text}
      onBlur={onCommit}
      onChange={(event) => onTextChange(event.currentTarget.value)}
      onKeyDown={(event) => {
        event.stopPropagation();
        if (event.key === 'Escape') {
          event.preventDefault();
          onDiscard();
        } else if (
          event.key === 'Enter' &&
          (event.metaKey || event.ctrlKey)
        ) {
          event.preventDefault();
          event.currentTarget.blur();
        }
      }}
      onPointerDown={(event) => event.stopPropagation()}
    />
  );
}

interface CaptureRenderingOutputBarProps {
  isRenderingOutput: boolean;
  selectionViewportRect: LogicalRect;
}

export function CaptureRenderingOutputBar({
  isRenderingOutput,
  selectionViewportRect,
}: CaptureRenderingOutputBarProps) {
  if (!isRenderingOutput) return null;

  return (
    <div
      className="absolute h-1 bg-white/80"
      style={{
        left: `${selectionViewportRect.x}px`,
        top: `${selectionViewportRect.y + selectionViewportRect.height}px`,
        width: `${selectionViewportRect.width}px`,
        zIndex: 2,
      }}
    />
  );
}

interface CaptureSelectionResizeHandlesProps {
  selectionViewportRect: LogicalRect;
  onResizeHandlePointerDown: (
    handle: SelectionHandle,
    event: PointerEvent<HTMLButtonElement>,
  ) => void;
}

export function CaptureSelectionResizeHandles({
  selectionViewportRect,
  onResizeHandlePointerDown,
}: CaptureSelectionResizeHandlesProps) {
  return (
    <div
      className="absolute pointer-events-none"
      style={{ ...rectStyle(selectionViewportRect), zIndex: 2 }}
    >
      {(['n', 'e', 's', 'w'] as const).map((handle) => (
        <button
          key={`edge-${handle}`}
          className={`pointer-events-auto absolute border-0 bg-transparent p-0 ${resizeEdgeClassNames[handle]}`}
          aria-label={`Resize selection edge ${handle}`}
          onPointerDown={(event) => onResizeHandlePointerDown(handle, event)}
        />
      ))}
      {SELECTION_HANDLES.map((handle) => (
        <button
          key={handle}
          type="button"
          className={`pointer-events-auto absolute z-[3] flex h-4 w-4 items-center justify-center border-0 bg-transparent p-0 ${resizeHandleClassNames[handle]}`}
          aria-label={`Resize selection ${handle}`}
          onPointerDown={(event) => onResizeHandlePointerDown(handle, event)}
        >
          <span className="pointer-events-none h-2 w-2 rounded-full border border-[#5b7fff] bg-white shadow-[0_1px_3px_rgba(15,23,42,0.3)]" />
        </button>
      ))}
    </div>
  );
}
