import { useRef, useState, type MouseEvent, type ReactNode } from "react";
import IconActionButton from "../../components/common/IconActionButton";
import {
  MAX_ANNOTATION_STROKE_WIDTH,
  MAX_TEXT_FONT_SIZE,
  MIN_ANNOTATION_STROKE_WIDTH,
  MIN_TEXT_FONT_SIZE,
  annotationColorToCss,
  type AnnotationColor,
  type AnnotationStyle,
  type AnnotationTool,
} from "./annotationStyle";
import {
  addAnnotationColorPreset,
  annotationColorFromHex,
  annotationColorToHex,
  annotationColorsEqual,
  removeAnnotationColorPreset,
  replaceAnnotationColorPreset,
} from "./annotationColorPresets";
import { getSaveCapturePointerAction } from "./captureActions";
import {
  getCaptureEditorCommandButtonClassName,
  getCaptureEditorDividerClassName,
  getCaptureEditorIconButtonClassName,
  getCaptureEditorToolbarClassName,
} from "./capturePresentation";
import type { Point } from "./types";

interface CaptureEditorToolbarProps {
  position: Point;
  width: number;
  activeAnnotationTool: AnnotationTool | null;
  annotationStyle: AnnotationStyle;
  annotationColorPresets: readonly AnnotationColor[];
  textFontSize: number;
  textDraftActive: boolean;
  isTextSizingActive: boolean;
  isFillModeActive: boolean;
  canUndo: boolean;
  canRedo: boolean;
  isRenderingOutput: boolean;
  onSelectMove: () => void;
  onToggleAnnotationTool: (tool: AnnotationTool) => void;
  onApplyAnnotationStyle: (
    nextStyle: AnnotationStyle,
    nextTextFontSize: number,
  ) => void;
  onUpdateAnnotationColorPresets: (
    colors: AnnotationColor[],
  ) => void | Promise<unknown>;
  onTextDraftFontSizeChange: (fontSize: number) => void;
  onCommitSizeDefault: (kind: "stroke" | "font", value: number) => void;
  onUndo: () => void;
  onRedo: () => void;
  onCancel: () => void | Promise<void>;
  onRunOcr: () => void | Promise<void>;
  onCopy: () => void | Promise<void>;
  onSave: () => void | Promise<void>;
  onQuickSave: () => void | Promise<void>;
  onPin: () => void | Promise<void>;
  onFavorite: () => void | Promise<void>;
}

interface AnnotationToolButton {
  tool: AnnotationTool;
  title: string;
  ariaLabel: string;
  icon: ReactNode;
}

const ANNOTATION_TOOL_BUTTONS: AnnotationToolButton[] = [
  {
    tool: "pen",
    title: "Pencil",
    ariaLabel: "Draw freehand annotation",
    icon: <PenIcon />,
  },
  {
    tool: "text",
    title: "Text",
    ariaLabel: "Add text annotation",
    icon: <TextIcon />,
  },
  {
    tool: "mosaic",
    title: "Mosaic",
    ariaLabel: "Draw mosaic annotation",
    icon: <MosaicIcon />,
  },
  {
    tool: "eraser",
    title: "Eraser",
    ariaLabel: "Erase annotation",
    icon: <EraserIcon />,
  },
];

const SHAPE_TOOLS: AnnotationToolButton[] = [
  {
    tool: "rectangle",
    title: "Rectangle",
    ariaLabel: "Draw rectangle annotation",
    icon: <RectangleIcon />,
  },
  {
    tool: "ellipse",
    title: "Ellipse / circle",
    ariaLabel: "Draw ellipse or circle annotation",
    icon: <EllipseIcon />,
  },
];

const ARROW_TOOLS: AnnotationToolButton[] = [
  {
    tool: "arrow",
    title: "Arrow",
    ariaLabel: "Draw arrow annotation",
    icon: <ArrowIcon />,
  },
  {
    tool: "line",
    title: "Line",
    ariaLabel: "Draw line annotation",
    icon: <LineIcon />,
  },
];

export function CaptureEditorToolbar({
  position,
  width,
  activeAnnotationTool,
  annotationStyle,
  annotationColorPresets,
  textFontSize,
  textDraftActive,
  isTextSizingActive,
  isFillModeActive,
  canUndo,
  canRedo,
  isRenderingOutput,
  onSelectMove,
  onToggleAnnotationTool,
  onApplyAnnotationStyle,
  onUpdateAnnotationColorPresets,
  onTextDraftFontSizeChange,
  onCommitSizeDefault,
  onUndo,
  onRedo,
  onCancel,
  onRunOcr,
  onCopy,
  onSave,
  onQuickSave,
  onPin,
  onFavorite,
}: CaptureEditorToolbarProps) {
  const [openPicker, setOpenPicker] = useState<
    "shape" | "arrow" | "color" | null
  >(null);
  const [visibleColorPresets, setVisibleColorPresets] = useState<
    readonly AnnotationColor[]
  >(() => [...annotationColorPresets]);
  const [colorDraft, setColorDraft] = useState(() =>
    annotationColorToHex(annotationStyle.color),
  );
  const [selectedColorPresetIndex, setSelectedColorPresetIndex] = useState<
    number | null
  >(null);
  const [colorPresetError, setColorPresetError] = useState<string | null>(null);
  const colorPresetMutationIdRef = useRef(0);
  const pendingSizeDefaultRef = useRef<{
    kind: "stroke" | "font";
    value: number;
  } | null>(null);
  const sizeValue = isTextSizingActive
    ? textFontSize
    : annotationStyle.strokeWidth;

  const handleSaveClick = (event: MouseEvent<HTMLButtonElement>) => {
    const action = getSaveCapturePointerAction(event);
    if (action === "quick-save") {
      void onQuickSave();
    } else {
      void onSave();
    }
  };

  const commitSizeDefault = () => {
    const pending = pendingSizeDefaultRef.current;
    if (!pending) return;
    pendingSizeDefaultRef.current = null;
    onCommitSizeDefault(pending.kind, pending.value);
  };

  const activeShapeTool = SHAPE_TOOLS.find(
    (button) => button.tool === activeAnnotationTool,
  );
  const activeArrowTool = ARROW_TOOLS.find(
    (button) => button.tool === activeAnnotationTool,
  );
  const primaryShapeTool = activeShapeTool ?? SHAPE_TOOLS[0];
  const primaryArrowTool = activeArrowTool ?? ARROW_TOOLS[0];
  const togglePicker = (picker: "shape" | "arrow") => {
    setOpenPicker((current) => (current === picker ? null : picker));
  };
  const selectPickerTool = (tool: AnnotationTool) => {
    if (activeAnnotationTool !== tool) onToggleAnnotationTool(tool);
    setOpenPicker(null);
  };
  const applyColor = (color: AnnotationColor) => {
    setColorDraft(annotationColorToHex(color));
    onApplyAnnotationStyle(
      {
        ...annotationStyle,
        color,
      },
      textFontSize,
    );
  };
  const openColorPicker = () => {
    if (openPicker === "color") {
      setOpenPicker(null);
      return;
    }

    const currentColor = annotationStyle.color;
    setColorPresetError(null);
    setVisibleColorPresets([...annotationColorPresets]);
    setColorDraft(annotationColorToHex(currentColor));
    const selectedIndex = annotationColorPresets.findIndex((color) =>
      annotationColorsEqual(color, currentColor),
    );
    setSelectedColorPresetIndex(selectedIndex >= 0 ? selectedIndex : null);
    setOpenPicker("color");
  };
  const updateColorDraft = (value: string) => {
    const color = annotationColorFromHex(value);
    if (!color) return;
    applyColor(color);
  };
  const persistColorPresets = async (
    colors: AnnotationColor[],
    nextSelectedIndex: number | null,
  ) => {
    const previousColors = visibleColorPresets;
    const previousSelectedIndex = selectedColorPresetIndex;
    const mutationId = ++colorPresetMutationIdRef.current;
    setVisibleColorPresets(colors);
    setSelectedColorPresetIndex(nextSelectedIndex);
    setColorPresetError(null);
    try {
      await onUpdateAnnotationColorPresets(colors);
    } catch {
      if (mutationId !== colorPresetMutationIdRef.current) return;
      setVisibleColorPresets(previousColors);
      setSelectedColorPresetIndex(previousSelectedIndex);
      setColorPresetError("颜色预设保存失败，请重试。");
    }
  };
  const addColorPreset = () => {
    const color = annotationColorFromHex(colorDraft);
    if (!color) return;
    const nextColors = addAnnotationColorPreset(visibleColorPresets, color);
    if (nextColors.length === visibleColorPresets.length) return;
    void persistColorPresets(nextColors, nextColors.length - 1);
  };
  const replaceColorPreset = () => {
    if (selectedColorPresetIndex === null) return;
    const color = annotationColorFromHex(colorDraft);
    if (!color) return;
    const nextColors = replaceAnnotationColorPreset(
      visibleColorPresets,
      selectedColorPresetIndex,
      color,
    );
    void persistColorPresets(nextColors, selectedColorPresetIndex);
  };
  const deleteColorPreset = () => {
    if (selectedColorPresetIndex === null) return;
    const nextColors = removeAnnotationColorPreset(
      visibleColorPresets,
      selectedColorPresetIndex,
    );
    void persistColorPresets(nextColors, null);
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
        if (textDraftActive && target.tagName !== "INPUT") {
          event.preventDefault();
        }
      }}
    >
      <IconActionButton
        className={getCaptureEditorIconButtonClassName(!activeAnnotationTool)}
        disabled={isRenderingOutput}
        title="Select and move"
        tooltipPlacement="bottom"
        aria-label="Select and move"
        onClick={() => {
          setOpenPicker(null);
          onSelectMove();
        }}
      >
        <PointerIcon />
      </IconActionButton>
      <ToolGroupButton
        label="Shapes"
        groupIcon={<ShapeGroupIcon />}
        primaryTool={primaryShapeTool}
        tools={SHAPE_TOOLS}
        activeTool={activeAnnotationTool}
        isOpen={openPicker === "shape"}
        disabled={isRenderingOutput}
        onActivate={selectPickerTool}
        onToggle={() => togglePicker("shape")}
      />
      <ToolGroupButton
        label="Arrows and lines"
        groupIcon={<ArrowIcon />}
        primaryTool={primaryArrowTool}
        tools={ARROW_TOOLS}
        activeTool={activeAnnotationTool}
        isOpen={openPicker === "arrow"}
        disabled={isRenderingOutput}
        onActivate={selectPickerTool}
        onToggle={() => togglePicker("arrow")}
      />
      {ANNOTATION_TOOL_BUTTONS.map((button) => (
        <IconActionButton
          key={button.tool}
          className={getCaptureEditorIconButtonClassName(
            activeAnnotationTool === button.tool,
          )}
          disabled={isRenderingOutput}
          title={button.title}
          tooltipPlacement="bottom"
          aria-label={button.ariaLabel}
          onClick={() => {
            setOpenPicker(null);
            onToggleAnnotationTool(button.tool);
          }}
        >
          {button.icon}
        </IconActionButton>
      ))}
      <div className={getCaptureEditorDividerClassName()} />
      <IconActionButton
        className={getCaptureEditorIconButtonClassName(false)}
        disabled={isRenderingOutput || !canUndo}
        title="Undo (Ctrl/Cmd+Z)"
        tooltipPlacement="bottom"
        aria-label="Undo annotation"
        onClick={onUndo}
      >
        <UndoIcon />
      </IconActionButton>
      <IconActionButton
        className={getCaptureEditorIconButtonClassName(false)}
        disabled={isRenderingOutput || !canRedo}
        title="Redo (Ctrl/Cmd+Y)"
        tooltipPlacement="bottom"
        aria-label="Redo annotation"
        onClick={onRedo}
      >
        <RedoIcon />
      </IconActionButton>
      <div className={getCaptureEditorDividerClassName()} />
      <div className="relative flex h-7 shrink-0">
        <IconActionButton
          className="h-7 w-7 shrink-0 rounded-[8px] border border-slate-200 bg-[#5b7fff] shadow-[0_0_0_2px_rgba(91,127,255,0.12)] disabled:cursor-not-allowed disabled:opacity-40"
          style={{
            backgroundColor: annotationColorToCss(annotationStyle.color),
          }}
          disabled={isRenderingOutput}
          title="Annotation color"
          tooltipPlacement="bottom"
          aria-label="Annotation color"
          aria-haspopup="dialog"
          aria-expanded={openPicker === "color"}
          onClick={openColorPicker}
        >
          <span className="sr-only">Annotation color</span>
        </IconActionButton>
        {openPicker === "color" && (
          <ColorPickerPopover
            colors={visibleColorPresets}
            draftColor={colorDraft}
            selectedIndex={selectedColorPresetIndex}
            error={colorPresetError}
            opensUpward={position.y + 292 > window.innerHeight}
            onClose={() => setOpenPicker(null)}
            onSelect={(color, index) => {
              setSelectedColorPresetIndex(index);
              applyColor(color);
            }}
            onDraftChange={updateColorDraft}
            onAdd={addColorPreset}
            onReplace={replaceColorPreset}
            onDelete={deleteColorPreset}
          />
        )}
      </div>
      <input
        className="h-7 w-14 accent-[#5b7fff] disabled:opacity-40"
        type="range"
        min={
          isTextSizingActive ? MIN_TEXT_FONT_SIZE : MIN_ANNOTATION_STROKE_WIDTH
        }
        max={
          isTextSizingActive ? MAX_TEXT_FONT_SIZE : MAX_ANNOTATION_STROKE_WIDTH
        }
        step={1}
        value={sizeValue}
        disabled={isRenderingOutput}
        title={
          isTextSizingActive ? "Text font size" : "Annotation stroke width"
        }
        aria-label={
          isTextSizingActive ? "Text font size" : "Annotation stroke width"
        }
        onInput={(event) => {
          const value = Number(event.currentTarget.value);
          pendingSizeDefaultRef.current = {
            kind: isTextSizingActive ? "font" : "stroke",
            value,
          };
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
        onPointerUp={commitSizeDefault}
        onKeyUp={commitSizeDefault}
        onBlur={commitSizeDefault}
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
        className={getCaptureEditorCommandButtonClassName("icon")}
        disabled={isRenderingOutput}
        title="Add to favorites"
        aria-label="Add capture to favorites"
        onClick={() => void onFavorite()}
      >
        <FavoriteIcon />
      </button>
      <button
        type="button"
        className={getCaptureEditorCommandButtonClassName("icon")}
        disabled={isRenderingOutput}
        title="Pin capture (Ctrl/Cmd+T)"
        aria-label="Pin capture"
        onClick={() => void onPin()}
      >
        <PinIcon />
      </button>
      <button
        type="button"
        className={getCaptureEditorCommandButtonClassName("icon")}
        disabled={isRenderingOutput}
        title="Save (Ctrl/Cmd+S, Shift-click for quick save)"
        aria-label="Save selection"
        onClick={handleSaveClick}
      >
        <SaveIcon />
      </button>
      <button
        type="button"
        className={getCaptureEditorCommandButtonClassName("primary")}
        disabled={isRenderingOutput}
        title="Copy (Cmd/Ctrl+C)"
        aria-label="Copy selection"
        onClick={() => {
          void onCopy();
        }}
      >
        <CopyIcon />
      </button>
    </div>
  );
}

function ColorPickerPopover({
  colors,
  draftColor,
  selectedIndex,
  error,
  opensUpward,
  onClose,
  onSelect,
  onDraftChange,
  onAdd,
  onReplace,
  onDelete,
}: {
  colors: readonly AnnotationColor[];
  draftColor: string;
  selectedIndex: number | null;
  error: string | null;
  opensUpward: boolean;
  onClose: () => void;
  onSelect: (color: AnnotationColor, index: number) => void;
  onDraftChange: (value: string) => void;
  onAdd: () => void;
  onReplace: () => void;
  onDelete: () => void;
}) {
  const systemPaletteInputRef = useRef<HTMLInputElement>(null);
  const draftAnnotationColor = annotationColorFromHex(draftColor);
  const duplicateIndex = draftAnnotationColor
    ? colors.findIndex((color) =>
        annotationColorsEqual(color, draftAnnotationColor),
      )
    : -1;
  const selectedColor =
    selectedIndex === null ? null : (colors[selectedIndex] ?? null);
  const canAdd = draftAnnotationColor !== null && duplicateIndex < 0;
  const canReplace =
    selectedIndex !== null &&
    draftAnnotationColor !== null &&
    selectedColor !== null &&
    !annotationColorsEqual(selectedColor, draftAnnotationColor) &&
    (duplicateIndex < 0 || duplicateIndex === selectedIndex);

  return (
    <div
      className={`absolute left-1/2 z-20 w-[248px] -translate-x-1/2 rounded-[12px] border border-slate-200 bg-white/95 p-3 text-slate-700 shadow-[0_14px_34px_rgba(15,23,42,0.22)] backdrop-blur-xl ${
        opensUpward ? "bottom-[calc(100%+3px)]" : "top-[calc(100%+3px)]"
      }`}
      role="dialog"
      aria-label="Annotation colors"
    >
      <div className="mb-2.5 flex items-center justify-between gap-3">
        <span className="text-[12px] font-semibold text-slate-800">颜色</span>
        <button
          type="button"
          className="flex h-5 w-5 items-center justify-center rounded text-[15px] font-normal text-slate-400 hover:bg-slate-100 hover:text-slate-700"
          aria-label="Close color palette"
          onClick={onClose}
        >
          ×
        </button>
      </div>

      {colors.length > 0 ? (
        <div className="grid grid-cols-7 gap-1.5" aria-label="Preset colors">
          {colors.map((color, index) => {
            const hexColor = annotationColorToHex(color);
            return (
              <button
                key={`${hexColor}-${index}`}
                type="button"
                className={`h-6 w-6 rounded-[6px] border border-black/10 shadow-sm transition-transform hover:scale-110 focus:outline-none ${
                  selectedIndex === index
                    ? "ring-2 ring-[#5b7fff] ring-offset-2"
                    : ""
                }`}
                style={{ backgroundColor: annotationColorToCss(color) }}
                title={hexColor}
                aria-label={`Use preset ${hexColor}`}
                onClick={() => onSelect(color, index)}
              />
            );
          })}
        </div>
      ) : (
        <div className="rounded-[8px] border border-dashed border-slate-200 px-3 py-4 text-center text-[11px] font-normal text-slate-400">
          暂无预设颜色
        </div>
      )}

      <div className="relative mt-3 h-9">
        <button
          type="button"
          className="flex h-full w-full items-center gap-2 rounded-[8px] border border-slate-200 bg-slate-50 px-2.5 text-left text-[11px] font-semibold text-slate-700 hover:bg-slate-100"
          aria-label="Open system color palette"
          aria-haspopup="dialog"
          onClick={() => systemPaletteInputRef.current?.click()}
        >
          <span
            className="h-5 w-5 rounded-full border border-white shadow-sm"
            style={{
              background:
                "conic-gradient(#ff4d4f, #faad14, #52c41a, #1890ff, #722ed1, #ff4d4f)",
            }}
            aria-hidden="true"
          />
          <span className="flex-1">系统调色板</span>
          <span className="font-mono text-[10px] text-slate-400">
            {draftColor}
          </span>
        </button>
        <input
          ref={systemPaletteInputRef}
          className="pointer-events-none absolute inset-0 h-full w-full opacity-0"
          type="color"
          value={draftColor}
          tabIndex={-1}
          aria-label="Color palette"
          onInput={(event) => onDraftChange(event.currentTarget.value)}
        />
      </div>

      <div className="mt-2.5 grid grid-cols-3 gap-1.5 border-t border-slate-100 pt-2.5">
        <button
          type="button"
          className="h-7 rounded-[7px] border border-slate-200 bg-white text-[10px] font-semibold text-slate-600 hover:bg-slate-50 disabled:cursor-not-allowed disabled:opacity-35"
          disabled={!canAdd}
          aria-label="Add preset color"
          onClick={onAdd}
        >
          新增
        </button>
        <button
          type="button"
          className="h-7 rounded-[7px] border border-slate-200 bg-white text-[10px] font-semibold text-slate-600 hover:bg-slate-50 disabled:cursor-not-allowed disabled:opacity-35"
          disabled={!canReplace}
          aria-label="Replace selected preset"
          onClick={onReplace}
        >
          修改
        </button>
        <button
          type="button"
          className="h-7 rounded-[7px] border border-red-200 bg-white text-[10px] font-semibold text-red-600 hover:bg-red-50 disabled:cursor-not-allowed disabled:opacity-35"
          disabled={selectedIndex === null}
          aria-label="Delete selected preset"
          onClick={onDelete}
        >
          删除
        </button>
      </div>
      {error && (
        <p className="mt-2 text-[10px] text-red-600" role="alert">
          {error}
        </p>
      )}
    </div>
  );
}

function PointerIcon() {
  return (
    <svg viewBox="0 0 24 24" className="h-[17px] w-[17px]" aria-hidden="true">
      <path
        d="M6.5 4.5v14.3l3.65-3.8 2.55 5 2.25-1.15-2.5-4.85h5.25L6.5 4.5Z"
        fill="none"
        stroke="currentColor"
        strokeLinejoin="round"
        strokeWidth="1.65"
      />
    </svg>
  );
}

function ShapeGroupIcon() {
  return (
    <svg viewBox="0 0 24 24" className="h-[17px] w-[17px]" aria-hidden="true">
      <rect
        x="4.5"
        y="5.5"
        width="10.5"
        height="9"
        rx="1.5"
        fill="none"
        stroke="currentColor"
        strokeWidth="1.65"
      />
      <ellipse
        cx="15.4"
        cy="14.6"
        rx="4.7"
        ry="3.8"
        fill="none"
        stroke="currentColor"
        strokeWidth="1.65"
      />
    </svg>
  );
}

function RectangleIcon() {
  return (
    <svg viewBox="0 0 24 24" className="h-[17px] w-[17px]" aria-hidden="true">
      <rect
        x="4.5"
        y="6"
        width="15"
        height="12"
        rx="2"
        fill="none"
        stroke="currentColor"
        strokeWidth="1.65"
      />
    </svg>
  );
}

function ChevronIcon() {
  return (
    <svg viewBox="0 0 24 24" className="ml-[-2px] h-2 w-2" aria-hidden="true">
      <path
        d="m7.5 9.5 4.5 4.5 4.5-4.5"
        fill="none"
        stroke="currentColor"
        strokeLinecap="round"
        strokeLinejoin="round"
        strokeWidth="1.65"
      />
    </svg>
  );
}

function EllipseIcon() {
  return (
    <svg viewBox="0 0 24 24" className="h-[17px] w-[17px]" aria-hidden="true">
      <ellipse
        cx="12"
        cy="12"
        rx="7.8"
        ry="5.8"
        fill="none"
        stroke="currentColor"
        strokeWidth="1.65"
      />
    </svg>
  );
}

function ArrowIcon() {
  return (
    <svg viewBox="0 0 24 24" className="h-[17px] w-[17px]" aria-hidden="true">
      <path
        d="m5.5 18.5 13-13M11.8 5.5h6.7v6.7"
        fill="none"
        stroke="currentColor"
        strokeLinecap="round"
        strokeLinejoin="round"
        strokeWidth="1.65"
      />
    </svg>
  );
}

function LineIcon() {
  return (
    <svg viewBox="0 0 24 24" className="h-[17px] w-[17px]" aria-hidden="true">
      <path
        d="m5.5 18.5 13-13"
        fill="none"
        stroke="currentColor"
        strokeLinecap="round"
        strokeWidth="1.65"
      />
    </svg>
  );
}

function PenIcon() {
  return (
    <svg viewBox="0 0 24 24" className="h-[17px] w-[17px]" aria-hidden="true">
      <path
        d="m5 19 1.15-4.45 9.15-9.15 3.3 3.3-9.15 9.15L5 19Z"
        fill="none"
        stroke="currentColor"
        strokeLinejoin="round"
        strokeWidth="1.65"
      />
      <path
        d="m13.9 6.8 3.3 3.3"
        fill="none"
        stroke="currentColor"
        strokeLinecap="round"
        strokeWidth="1.65"
      />
      <path
        d="m5 19 4.45-1.15-3.3-3.3L5 19Z"
        fill="none"
        stroke="currentColor"
        strokeLinejoin="round"
        strokeWidth="1.65"
      />
    </svg>
  );
}

function TextIcon() {
  return (
    <svg viewBox="0 0 24 24" className="h-[17px] w-[17px]" aria-hidden="true">
      <path
        d="M6.5 5.5h11M12 5.5v13M8.8 18.5h6.4"
        fill="none"
        stroke="currentColor"
        strokeLinecap="round"
        strokeWidth="1.65"
      />
    </svg>
  );
}

function MosaicIcon() {
  return (
    <svg viewBox="0 0 24 24" className="h-[17px] w-[17px]" aria-hidden="true">
      <rect
        x="4.5"
        y="4.5"
        width="15"
        height="15"
        rx="1.3"
        fill="none"
        stroke="currentColor"
        strokeWidth="1.65"
      />
      <path
        d="M9.5 4.5v15M14.5 4.5v15M4.5 9.5h15M4.5 14.5h15"
        fill="none"
        stroke="currentColor"
        strokeWidth="1.65"
        opacity="0.55"
      />
      <path
        d="M4.5 4.5h5v5h-5zM14.5 4.5h5v5h-5zM9.5 9.5h5v5h-5zM4.5 14.5h5v5h-5zM14.5 14.5h5v5h-5z"
        fill="currentColor"
      />
    </svg>
  );
}

function EraserIcon() {
  return (
    <svg viewBox="0 0 24 24" className="h-[17px] w-[17px]" aria-hidden="true">
      <path
        d="m5 15.2 8.7-8.7 4.8 4.8-7.2 7.2H8.2L5 15.2Z"
        fill="none"
        stroke="currentColor"
        strokeLinejoin="round"
        strokeWidth="1.65"
      />
      <path
        d="m10.1 10.1 4.8 4.8M11.3 18.5h7.2"
        fill="none"
        stroke="currentColor"
        strokeLinecap="round"
        strokeWidth="1.65"
      />
    </svg>
  );
}

function SaveIcon() {
  return (
    <svg viewBox="0 0 24 24" className="h-[17px] w-[17px]" aria-hidden="true">
      <path
        d="M12 4.5v10M8.5 11l3.5 3.5 3.5-3.5M5 19h14"
        fill="none"
        stroke="currentColor"
        strokeLinecap="round"
        strokeLinejoin="round"
        strokeWidth="1.65"
      />
    </svg>
  );
}

function CopyIcon() {
  return (
    <svg viewBox="0 0 24 24" className="h-[17px] w-[17px]" aria-hidden="true">
      <rect
        x="8"
        y="7.5"
        width="11.5"
        height="12"
        rx="2"
        fill="none"
        stroke="currentColor"
        strokeLinejoin="round"
        strokeWidth="1.65"
      />
      <path
        d="M15.5 7.5V6A1.8 1.8 0 0 0 13.7 4.2H6A1.8 1.8 0 0 0 4.2 6v8A1.8 1.8 0 0 0 6 15.8h2"
        fill="none"
        stroke="currentColor"
        strokeLinecap="round"
        strokeLinejoin="round"
        strokeWidth="1.65"
      />
    </svg>
  );
}

function UndoIcon() {
  return (
    <svg viewBox="0 0 24 24" className="h-[17px] w-[17px]" aria-hidden="true">
      <path
        d="M8.5 7.5H5V4M5.4 7.2a7.5 7.5 0 1 1-.1 9.8"
        fill="none"
        stroke="currentColor"
        strokeLinecap="round"
        strokeLinejoin="round"
        strokeWidth="1.65"
      />
    </svg>
  );
}

function RedoIcon() {
  return (
    <svg viewBox="0 0 24 24" className="h-[17px] w-[17px]" aria-hidden="true">
      <path
        d="M15.5 7.5H19V4M18.6 7.2a7.5 7.5 0 1 0 .1 9.8"
        fill="none"
        stroke="currentColor"
        strokeLinecap="round"
        strokeLinejoin="round"
        strokeWidth="1.65"
      />
    </svg>
  );
}

function PinIcon() {
  return (
    <svg viewBox="0 0 24 24" className="h-[17px] w-[17px]" aria-hidden="true">
      <path
        d="m14.3 3.4 6.3 6.3-2 2-1.6-1.6-3.2 3.2.7 2.7-1.4 1.4-3.6-3.6-5.2 5.2-1-1 5.2-5.2-3.6-3.6 1.4-1.4 2.7.7 3.2-3.2-1.6-1.6 2-2Z"
        fill="currentColor"
      />
    </svg>
  );
}

function FavoriteIcon() {
  return (
    <svg viewBox="0 0 24 24" className="h-[17px] w-[17px]" aria-hidden="true">
      <path
        d="M6.2 4.5h11.6v15l-5.8-3.4-5.8 3.4v-15Z"
        fill="none"
        stroke="currentColor"
        strokeLinejoin="round"
        strokeWidth="1.65"
      />
      <path
        d="m12 7.2 1.1 2.2 2.4.3-1.7 1.7.4 2.4-2.2-1.1-2.2 1.1.4-2.4-1.7-1.7 2.4-.3L12 7.2Z"
        fill="currentColor"
      />
    </svg>
  );
}

function ToolGroupButton({
  label,
  groupIcon,
  primaryTool,
  tools,
  activeTool,
  isOpen,
  disabled,
  onActivate,
  onToggle,
}: {
  label: string;
  groupIcon: ReactNode;
  primaryTool: AnnotationToolButton;
  tools: AnnotationToolButton[];
  activeTool: AnnotationTool | null;
  isOpen: boolean;
  disabled: boolean;
  onActivate: (tool: AnnotationTool) => void;
  onToggle: () => void;
}) {
  const isActive = tools.some((tool) => tool.tool === activeTool);

  return (
    <div className="relative flex h-7 shrink-0">
      <IconActionButton
        className={getCaptureEditorIconButtonClassName(isActive, "w-8 px-0")}
        disabled={disabled}
        title={label}
        tooltipPlacement="bottom"
        aria-label={label}
        aria-expanded={isOpen}
        onClick={() => {
          if (!isActive) onActivate(primaryTool.tool);
          onToggle();
        }}
      >
        {groupIcon}
        <ChevronIcon />
      </IconActionButton>
      {isOpen && (
        <ToolPicker
          label={label}
          tools={tools}
          activeTool={activeTool}
          onSelect={onActivate}
        />
      )}
    </div>
  );
}

function ToolPicker({
  label,
  tools,
  activeTool,
  onSelect,
}: {
  label: string;
  tools: AnnotationToolButton[];
  activeTool: AnnotationTool | null;
  onSelect: (tool: AnnotationTool) => void;
}) {
  return (
    <div
      className="absolute left-0 top-[calc(100%+6px)] z-10 flex min-w-max gap-1 rounded-[9px] border border-slate-200 bg-white/95 p-1 shadow-[0_8px_20px_rgba(15,23,42,0.2)] backdrop-blur-xl"
      role="menu"
      aria-label={label}
    >
      {tools.map((tool) => (
        <IconActionButton
          key={tool.tool}
          className={getCaptureEditorIconButtonClassName(
            activeTool === tool.tool,
          )}
          title={tool.title}
          tooltipPlacement="bottom"
          aria-label={tool.ariaLabel}
          onClick={() => onSelect(tool.tool)}
        >
          {tool.icon}
        </IconActionButton>
      ))}
    </div>
  );
}
