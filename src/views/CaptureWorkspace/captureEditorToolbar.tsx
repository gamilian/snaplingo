import type { MouseEvent, ReactNode } from 'react';
import IconActionButton from '../../components/common/IconActionButton';
import {
  ANNOTATION_COLORS,
  MAX_ANNOTATION_STROKE_WIDTH,
  MAX_TEXT_FONT_SIZE,
  MIN_ANNOTATION_STROKE_WIDTH,
  MIN_TEXT_FONT_SIZE,
  annotationColorToCss,
  type AnnotationColor,
  type AnnotationStyle,
  type AnnotationTool,
} from './annotationStyle';
import { getSaveCapturePointerAction } from './captureActions';
import {
  getCaptureEditorCommandButtonClassName,
  getCaptureEditorDividerClassName,
  getCaptureEditorIconButtonClassName,
  getCaptureEditorToolbarClassName,
} from './capturePresentation';
import type { Point } from './types';

interface CaptureEditorToolbarProps {
  position: Point;
  width: number;
  activeAnnotationTool: AnnotationTool | null;
  annotationStyle: AnnotationStyle;
  textFontSize: number;
  textDraftActive: boolean;
  isTextSizingActive: boolean;
  isFillModeActive: boolean;
  isRenderingOutput: boolean;
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

interface AnnotationToolButton {
  tool: AnnotationTool;
  title: string;
  ariaLabel: string;
  icon: ReactNode;
}

const ANNOTATION_TOOL_BUTTONS: AnnotationToolButton[] = [
  {
    tool: 'rectangle',
    title: 'Rectangle',
    ariaLabel: 'Draw rectangle annotation',
    icon: <RectangleIcon />,
  },
  {
    tool: 'ellipse',
    title: 'Ellipse',
    ariaLabel: 'Draw ellipse annotation',
    icon: <EllipseIcon />,
  },
  {
    tool: 'arrow',
    title: 'Arrow',
    ariaLabel: 'Draw arrow annotation',
    icon: <ArrowIcon />,
  },
  {
    tool: 'line',
    title: 'Line',
    ariaLabel: 'Draw line annotation',
    icon: <LineIcon />,
  },
  {
    tool: 'pen',
    title: 'Pen',
    ariaLabel: 'Draw freehand annotation',
    icon: <PenIcon />,
  },
  {
    tool: 'text',
    title: 'Text',
    ariaLabel: 'Add text annotation',
    icon: <TextIcon />,
  },
  {
    tool: 'mosaic',
    title: 'Mosaic',
    ariaLabel: 'Draw mosaic annotation',
    icon: <MosaicIcon />,
  },
  {
    tool: 'blur',
    title: 'Blur',
    ariaLabel: 'Draw blur annotation',
    icon: <BlurIcon />,
  },
  {
    tool: 'eraser',
    title: 'Eraser',
    ariaLabel: 'Erase annotation',
    icon: <EraserIcon />,
  },
];

export function CaptureEditorToolbar({
  position,
  width,
  activeAnnotationTool,
  annotationStyle,
  textFontSize,
  textDraftActive,
  isTextSizingActive,
  isFillModeActive,
  isRenderingOutput,
  onSelectMove,
  onToggleAnnotationTool,
  onApplyAnnotationStyle,
  onTextDraftFontSizeChange,
  onCancel,
  onRunOcr,
  onCopy,
  onSave,
  onQuickSave,
}: CaptureEditorToolbarProps) {
  const sizeValue = isTextSizingActive
    ? textFontSize
    : annotationStyle.strokeWidth;

  const handleSaveClick = (event: MouseEvent<HTMLButtonElement>) => {
    const action = getSaveCapturePointerAction(event);
    if (action === 'quick-save') {
      void onQuickSave();
    } else {
      void onSave();
    }
  };

  return (
    <div
      className={getCaptureEditorToolbarClassName()}
      style={{
        left: `${position.x}px`,
        top: `${position.y}px`,
        width: `${width}px`,
        zIndex: 2,
      }}
      onPointerDown={(event) => {
        event.stopPropagation();
        const target = event.target as HTMLElement;
        if (textDraftActive && target.tagName !== 'INPUT') {
          event.preventDefault();
        }
      }}
    >
      <IconActionButton
        className={getCaptureEditorIconButtonClassName(!activeAnnotationTool)}
        disabled={isRenderingOutput}
        title="Select and move"
        aria-label="Select and move"
        onClick={onSelectMove}
      >
        <PointerIcon />
      </IconActionButton>
      {ANNOTATION_TOOL_BUTTONS.map((button) => (
        <IconActionButton
          key={button.tool}
          className={getCaptureEditorIconButtonClassName(
            activeAnnotationTool === button.tool,
          )}
          disabled={isRenderingOutput}
          title={button.title}
          aria-label={button.ariaLabel}
          onClick={() => onToggleAnnotationTool(button.tool)}
        >
          {button.icon}
        </IconActionButton>
      ))}
      <div className={getCaptureEditorDividerClassName()} />
      <IconActionButton
        className="h-7 w-7 shrink-0 rounded-[8px] border border-slate-200 bg-[#5b7fff] shadow-[0_0_0_2px_rgba(91,127,255,0.12)] disabled:cursor-not-allowed disabled:opacity-40"
        style={{ backgroundColor: annotationColorToCss(annotationStyle.color) }}
        disabled={isRenderingOutput}
        title="Annotation color"
        aria-label="Annotation color"
        onClick={() => {
          onApplyAnnotationStyle(
            {
              ...annotationStyle,
              color: nextAnnotationColor(annotationStyle.color),
            },
            textFontSize,
          );
        }}
      >
        <span className="sr-only">Annotation color</span>
      </IconActionButton>
      <input
        className="h-7 w-14 accent-[#5b7fff] disabled:opacity-40"
        type="range"
        min={
          isTextSizingActive
            ? MIN_TEXT_FONT_SIZE
            : MIN_ANNOTATION_STROKE_WIDTH
        }
        max={
          isTextSizingActive
            ? MAX_TEXT_FONT_SIZE
            : MAX_ANNOTATION_STROKE_WIDTH
        }
        step={1}
        value={sizeValue}
        disabled={isRenderingOutput}
        title={isTextSizingActive ? 'Text font size' : 'Annotation stroke width'}
        aria-label={
          isTextSizingActive ? 'Text font size' : 'Annotation stroke width'
        }
        onChange={(event) => {
          const value = Number(event.currentTarget.value);
          if (textDraftActive) {
            onTextDraftFontSizeChange(value);
            return;
          }

          if (isTextSizingActive) {
            onApplyAnnotationStyle(annotationStyle, value);
            return;
          }

          onApplyAnnotationStyle(
            {
              ...annotationStyle,
              strokeWidth: value,
            },
            textFontSize,
          );
        }}
      />
      <input
        className="h-4 w-4 accent-[#5b7fff] disabled:opacity-40"
        type="checkbox"
        checked={annotationStyle.filled}
        disabled={isRenderingOutput || !isFillModeActive}
        title="Fill shape"
        aria-label="Fill shape"
        onChange={(event) => {
          onApplyAnnotationStyle(
            {
              ...annotationStyle,
              filled: event.currentTarget.checked,
            },
            textFontSize,
          );
        }}
      />
      <div className={getCaptureEditorDividerClassName()} />
      <button
        type="button"
        className={getCaptureEditorCommandButtonClassName()}
        disabled={isRenderingOutput}
        title="Cancel (Esc)"
        aria-label="Cancel capture"
        onClick={() => {
          void onCancel();
        }}
      >
        ESC
      </button>
      <button
        type="button"
        className={getCaptureEditorCommandButtonClassName()}
        disabled={isRenderingOutput}
        title="OCR"
        aria-label="Run OCR"
        onClick={() => {
          void onRunOcr();
        }}
      >
        OCR
      </button>
      <button
        type="button"
        className={getCaptureEditorCommandButtonClassName('icon')}
        disabled={isRenderingOutput}
        title="Copy (Ctrl/Cmd+C)"
        aria-label="Copy selection"
        onClick={() => {
          void onCopy();
        }}
      >
        <CopyIcon />
      </button>
      <button
        type="button"
        className={getCaptureEditorCommandButtonClassName('icon')}
        disabled={isRenderingOutput}
        title="Save (Ctrl/Cmd+S, Shift-click for quick save)"
        aria-label="Save selection"
        onClick={handleSaveClick}
      >
        <SaveIcon />
      </button>
      <button
        type="button"
        className={getCaptureEditorCommandButtonClassName('primary')}
        disabled={isRenderingOutput}
        title="Finish (Enter)"
        aria-label="Finish capture"
        onClick={() => {
          void onCopy();
        }}
      >
        <CheckIcon />
      </button>
    </div>
  );
}

function sameAnnotationColor(a: AnnotationColor, b: AnnotationColor) {
  return a.every((channel, index) => channel === b[index]);
}

function nextAnnotationColor(currentColor: AnnotationColor) {
  const currentIndex = ANNOTATION_COLORS.findIndex((color) =>
    sameAnnotationColor(currentColor, color),
  );

  return ANNOTATION_COLORS[
    (Math.max(0, currentIndex) + 1) % ANNOTATION_COLORS.length
  ];
}

function PointerIcon() {
  return (
    <svg viewBox="0 0 24 24" className="h-[17px] w-[17px]" aria-hidden="true">
      <path
        d="M7 4.8v13.9l3.2-3.5 2.4 4.9 2.3-1.1-2.4-4.8h5.1L7 4.8Z"
        fill="currentColor"
      />
    </svg>
  );
}

function RectangleIcon() {
  return (
    <svg viewBox="0 0 24 24" className="h-[17px] w-[17px]" aria-hidden="true">
      <rect
        x="5"
        y="8"
        width="14"
        height="8"
        fill="none"
        stroke="currentColor"
        strokeWidth="1.8"
      />
    </svg>
  );
}

function EllipseIcon() {
  return (
    <svg viewBox="0 0 24 24" className="h-[17px] w-[17px]" aria-hidden="true">
      <circle
        cx="12"
        cy="12"
        r="7"
        fill="none"
        stroke="currentColor"
        strokeWidth="1.8"
      />
    </svg>
  );
}

function ArrowIcon() {
  return (
    <svg viewBox="0 0 24 24" className="h-[17px] w-[17px]" aria-hidden="true">
      <path
        d="M5 12h13m-5-5 5 5-5 5"
        fill="none"
        stroke="currentColor"
        strokeLinecap="round"
        strokeLinejoin="round"
        strokeWidth="1.8"
      />
    </svg>
  );
}

function LineIcon() {
  return (
    <svg viewBox="0 0 24 24" className="h-[17px] w-[17px]" aria-hidden="true">
      <path
        d="M5 12h14"
        fill="none"
        stroke="currentColor"
        strokeLinecap="round"
        strokeWidth="1.8"
      />
    </svg>
  );
}

function PenIcon() {
  return (
    <svg viewBox="0 0 24 24" className="h-[17px] w-[17px]" aria-hidden="true">
      <path
        d="M5 17.5 15.7 6.8l2.5 2.5L7.5 20H5v-2.5Z"
        fill="none"
        stroke="currentColor"
        strokeLinejoin="round"
        strokeWidth="1.8"
      />
      <path
        d="m14.5 8 1.5-1.5a1.8 1.8 0 0 1 2.5 2.5L17 10.5"
        fill="none"
        stroke="currentColor"
        strokeLinecap="round"
        strokeWidth="1.8"
      />
    </svg>
  );
}

function TextIcon() {
  return (
    <svg
      viewBox="0 0 24 24"
      className="h-[17px] w-[17px]"
      aria-hidden="true"
    >
      <path
        d="M7 5h10M12 5v14"
        fill="none"
        stroke="currentColor"
        strokeLinecap="round"
        strokeWidth="1.8"
      />
    </svg>
  );
}

function MosaicIcon() {
  return (
    <svg viewBox="0 0 24 24" className="h-[17px] w-[17px]" aria-hidden="true">
      <path
        d="M5 5h14v14H5V5Zm4 0v14M15 5v14M5 9h14M5 15h14"
        fill="none"
        stroke="currentColor"
        strokeWidth="1.6"
      />
    </svg>
  );
}

function BlurIcon() {
  return (
    <svg viewBox="0 0 24 24" className="h-[17px] w-[17px]" aria-hidden="true">
      <circle
        cx="12"
        cy="12"
        r="7"
        fill="none"
        stroke="currentColor"
        strokeWidth="1.8"
      />
      <path d="M12 5a7 7 0 0 1 0 14V5Z" fill="currentColor" opacity="0.45" />
    </svg>
  );
}

function EraserIcon() {
  return (
    <svg viewBox="0 0 24 24" className="h-[17px] w-[17px]" aria-hidden="true">
      <path
        d="m5 15 8.6-8.6a2 2 0 0 1 2.8 0l2.2 2.2a2 2 0 0 1 0 2.8L12 18H8l-3-3Z"
        fill="none"
        stroke="currentColor"
        strokeLinejoin="round"
        strokeWidth="1.8"
      />
      <path
        d="M12 18h7"
        fill="none"
        stroke="currentColor"
        strokeLinecap="round"
        strokeWidth="1.8"
      />
    </svg>
  );
}

function CopyIcon() {
  return (
    <svg
      viewBox="0 0 24 24"
      className="h-[17px] w-[17px]"
      aria-hidden="true"
    >
      <rect
        x="8"
        y="8"
        width="11"
        height="11"
        rx="2"
        fill="none"
        stroke="currentColor"
        strokeWidth="1.8"
      />
      <path
        d="M16 8V6a2 2 0 0 0-2-2H6a2 2 0 0 0-2 2v8a2 2 0 0 0 2 2h2"
        fill="none"
        stroke="currentColor"
        strokeLinecap="round"
        strokeLinejoin="round"
        strokeWidth="1.8"
      />
    </svg>
  );
}

function SaveIcon() {
  return (
    <svg
      viewBox="0 0 24 24"
      className="h-[17px] w-[17px]"
      aria-hidden="true"
    >
      <path
        d="M12 4v11m-4-4 4 4 4-4M5 19h14"
        fill="none"
        stroke="currentColor"
        strokeLinecap="round"
        strokeLinejoin="round"
        strokeWidth="1.8"
      />
    </svg>
  );
}

function CheckIcon() {
  return (
    <svg
      viewBox="0 0 24 24"
      className="h-[17px] w-[17px]"
      aria-hidden="true"
    >
      <path
        d="m6 12 4 4 8-9"
        fill="none"
        stroke="currentColor"
        strokeLinecap="round"
        strokeLinejoin="round"
        strokeWidth="1.8"
      />
    </svg>
  );
}
