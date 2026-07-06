import type { PointerEvent, Ref } from 'react';
import {
  annotationColorToCss,
  arrowHeadPoints,
  type AnnotationStyle,
} from './annotationStyle';
import type { SelectionHandle } from './selection';
import type { TextAnnotationDraft } from './textAnnotationDraft';
import type { AnnotationCommand, LogicalRect, Point } from './types';

const SELECTION_HANDLES: SelectionHandle[] = ['nw', 'n', 'ne', 'e', 'se', 's', 'sw', 'w'];

const resizeHandleClassNames: Record<SelectionHandle, string> = {
  nw: '-left-1.5 -top-1.5 cursor-nwse-resize',
  n: 'left-1/2 -top-1.5 -translate-x-1/2 cursor-ns-resize',
  ne: '-right-1.5 -top-1.5 cursor-nesw-resize',
  e: '-right-1.5 top-1/2 -translate-y-1/2 cursor-ew-resize',
  se: '-bottom-1.5 -right-1.5 cursor-nwse-resize',
  s: '-bottom-1.5 left-1/2 -translate-x-1/2 cursor-ns-resize',
  sw: '-bottom-1.5 -left-1.5 cursor-nesw-resize',
  w: '-left-1.5 top-1/2 -translate-y-1/2 cursor-ew-resize',
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

function svgPolylinePoints(points: Point[]) {
  return points.map((point) => `${point.x},${point.y}`).join(' ');
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

interface CaptureDraftAnnotationOverlayProps {
  draftAnnotation: AnnotationCommand | null;
  selectionViewportRect: LogicalRect;
}

export function CaptureDraftAnnotationOverlay({
  draftAnnotation,
  selectionViewportRect,
}: CaptureDraftAnnotationOverlayProps) {
  if (!draftAnnotation) return null;

  if (draftAnnotation.type === 'rectangle') {
    return (
      <div
        className="pointer-events-none absolute"
        style={{
          ...rectStyle(
            annotationRectToViewportRect(
              draftAnnotation.rect,
              selectionViewportRect,
            ),
          ),
          border: `${draftAnnotation.stroke_width}px solid ${annotationColorToCss(
            draftAnnotation.color,
          )}`,
          backgroundColor: draftAnnotation.filled
            ? annotationColorToCss(draftAnnotation.color)
            : 'transparent',
        }}
      />
    );
  }

  if (draftAnnotation.type === 'ellipse') {
    return (
      <svg
        className="pointer-events-none absolute overflow-visible"
        style={rectStyle(selectionViewportRect)}
        viewBox={`0 0 ${selectionViewportRect.width} ${selectionViewportRect.height}`}
        fill={
          draftAnnotation.filled
            ? annotationColorToCss(draftAnnotation.color)
            : 'none'
        }
      >
        <ellipse
          cx={draftAnnotation.rect.x + draftAnnotation.rect.width / 2}
          cy={draftAnnotation.rect.y + draftAnnotation.rect.height / 2}
          rx={draftAnnotation.rect.width / 2}
          ry={draftAnnotation.rect.height / 2}
          stroke={annotationColorToCss(draftAnnotation.color)}
          strokeWidth={draftAnnotation.stroke_width}
        />
      </svg>
    );
  }

  if (draftAnnotation.type === 'mosaic') {
    return (
      <div
        className="pointer-events-none absolute border border-white/70 bg-black/35"
        style={{
          ...rectStyle(
            annotationRectToViewportRect(
              draftAnnotation.rect,
              selectionViewportRect,
            ),
          ),
          backgroundImage:
            'linear-gradient(45deg, rgba(255,255,255,0.2) 25%, transparent 25%, transparent 75%, rgba(255,255,255,0.2) 75%), linear-gradient(45deg, rgba(255,255,255,0.2) 25%, transparent 25%, transparent 75%, rgba(255,255,255,0.2) 75%)',
          backgroundPosition: '0 0, 4px 4px',
          backgroundSize: '8px 8px',
        }}
      />
    );
  }

  if (draftAnnotation.type === 'blur') {
    return (
      <div
        className="pointer-events-none absolute border border-white/70 bg-white/10"
        style={{
          ...rectStyle(
            annotationRectToViewportRect(
              draftAnnotation.rect,
              selectionViewportRect,
            ),
          ),
          backdropFilter: `blur(${draftAnnotation.radius}px)`,
        }}
      />
    );
  }

  if (draftAnnotation.type === 'line') {
    return (
      <svg
        className="pointer-events-none absolute overflow-visible"
        style={rectStyle(selectionViewportRect)}
        viewBox={`0 0 ${selectionViewportRect.width} ${selectionViewportRect.height}`}
        fill="none"
      >
        <line
          x1={draftAnnotation.start.x}
          y1={draftAnnotation.start.y}
          x2={draftAnnotation.end.x}
          y2={draftAnnotation.end.y}
          stroke={annotationColorToCss(draftAnnotation.color)}
          strokeWidth={draftAnnotation.stroke_width}
          strokeLinecap="round"
        />
      </svg>
    );
  }

  if (draftAnnotation.type === 'arrow') {
    const points = arrowHeadPoints(
      draftAnnotation.start,
      draftAnnotation.end,
      draftAnnotation.stroke_width,
    );

    return (
      <svg
        className="pointer-events-none absolute overflow-visible"
        style={rectStyle(selectionViewportRect)}
        viewBox={`0 0 ${selectionViewportRect.width} ${selectionViewportRect.height}`}
        fill="none"
      >
        <line
          x1={draftAnnotation.start.x}
          y1={draftAnnotation.start.y}
          x2={draftAnnotation.end.x}
          y2={draftAnnotation.end.y}
          stroke={annotationColorToCss(draftAnnotation.color)}
          strokeWidth={draftAnnotation.stroke_width}
          strokeLinecap="round"
        />
        {points && (
          <polygon
            points={points}
            fill={annotationColorToCss(draftAnnotation.color)}
          />
        )}
      </svg>
    );
  }

  if (draftAnnotation.type === 'freehand' || draftAnnotation.type === 'highlight') {
    return (
      <svg
        className="pointer-events-none absolute overflow-visible"
        style={rectStyle(selectionViewportRect)}
        viewBox={`0 0 ${selectionViewportRect.width} ${selectionViewportRect.height}`}
        fill="none"
      >
        <polyline
          points={svgPolylinePoints(draftAnnotation.points)}
          stroke={annotationColorToCss(draftAnnotation.color)}
          strokeWidth={draftAnnotation.stroke_width}
          strokeLinecap="round"
          strokeLinejoin="round"
        />
      </svg>
    );
  }

  if (draftAnnotation.type === 'text') {
    return (
      <div
        className="pointer-events-none absolute whitespace-pre"
        style={{
          left: `${selectionViewportRect.x + draftAnnotation.position.x}px`,
          top: `${
            selectionViewportRect.y +
            draftAnnotation.position.y -
            draftAnnotation.font_size
          }px`,
          color: annotationColorToCss(draftAnnotation.color),
          fontSize: `${draftAnnotation.font_size}px`,
          lineHeight: 1,
        }}
      >
        {draftAnnotation.text}
      </div>
    );
  }

  return null;
}

interface CaptureSelectedAnnotationBoundsOverlayProps {
  selectedAnnotationBounds: LogicalRect | null;
  selectionViewportRect: LogicalRect;
}

export function CaptureSelectedAnnotationBoundsOverlay({
  selectedAnnotationBounds,
  selectionViewportRect,
}: CaptureSelectedAnnotationBoundsOverlayProps) {
  if (!selectedAnnotationBounds) return null;

  return (
    <div
      className="pointer-events-none absolute border border-dashed border-white shadow-[0_0_0_1px_rgba(0,0,0,0.55)]"
      style={rectStyle(
        annotationRectToViewportRect(
          selectedAnnotationBounds,
          selectionViewportRect,
        ),
      )}
    />
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
  return (
    <textarea
      ref={inputRef}
      data-screenshot-text-draft="true"
      className="absolute resize-none overflow-hidden border border-white/70 bg-black/15 px-1 py-0 text-left outline-none ring-1 ring-black/35"
      style={{
        left: `${selectionViewportRect.x + textDraft.position.x}px`,
        top: `${
          selectionViewportRect.y +
          textDraft.position.y -
          textDraft.fontSize
        }px`,
        width: `${Math.max(160, selectionViewportRect.width - textDraft.position.x)}px`,
        minHeight: `${Math.ceil(textDraft.fontSize * 1.35)}px`,
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
      {SELECTION_HANDLES.map((handle) => (
        <button
          key={handle}
          className={`pointer-events-auto absolute h-3 w-3 rounded-full border border-[#5b7fff] bg-white shadow-[0_2px_7px_rgba(91,127,255,0.35)] ${resizeHandleClassNames[handle]}`}
          aria-label={`Resize selection ${handle}`}
          onPointerDown={(event) => onResizeHandlePointerDown(handle, event)}
        />
      ))}
    </div>
  );
}
