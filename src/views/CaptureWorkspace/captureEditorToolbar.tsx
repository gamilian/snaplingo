import { annotationColorToCss } from '../../components/common/annotationColorPresentation';
import { useRef, useState, type MouseEvent, type ReactNode, type RefObject } from "react";
import IconActionButton from "../../components/common/IconActionButton";
import {
  MAX_ANNOTATION_STROKE_WIDTH,
  MAX_TEXT_FONT_SIZE,
  MIN_ANNOTATION_STROKE_WIDTH,
  MIN_TEXT_FONT_SIZE,
  DEFAULT_BLUR_RADIUS,
  MIN_BLUR_RADIUS,
  MAX_BLUR_RADIUS,
  nextTextFontSize,
  type AnnotationColor,
  type AnnotationStyle,
  type AnnotationTool,
} from "../../application/capture-workspace/annotationStyle";
import {
  addAnnotationColorPreset,
  annotationColorFromHex,
  annotationColorToHex,
  annotationColorsEqual,
  removeAnnotationColorPreset,
  replaceAnnotationColorPreset,
} from "./annotationColorPresets";
import { getSaveCapturePointerAction } from "../../application/capture-workspace/captureActions";
import {
  getCaptureEditorCommandButtonClassName,
  getCaptureEditorDividerClassName,
  getCaptureEditorIconButtonClassName,
  getCaptureEditorToolbarClassName,
} from "./capturePresentation";
import type { Point } from "./types";

interface CaptureEditorToolbarProps {
  toolbarRef?: RefObject<HTMLDivElement>;
  position: Point;
  width: number;
  activeAnnotationTool: AnnotationTool | null;
  annotationStyle: AnnotationStyle;
  annotationColorPresets: readonly AnnotationColor[];
  fontFamilies?: readonly string[];
  selectedAnnotationType?: string;
  uiScale?: number;
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
  onClear: () => void;
  onCancel: () => void | Promise<void>;
  onBeginScrollingCapture: () => void | Promise<void>;
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
    tool: "eraser",
    title: "Eraser",
    ariaLabel: "Erase annotation",
    icon: <EraserIcon />,
  },
  { tool: "highlight", title: "Highlighter", ariaLabel: "Draw highlight annotation", icon: <EffectIcon kind="highlight" /> },
];

const REDACTION_TOOLS: AnnotationToolButton[] = [
  {
    tool: "mosaic",
    title: "Mosaic",
    ariaLabel: "Draw mosaic annotation",
    icon: <MosaicIcon />,
  },
  { tool: "blur", title: "Blur", ariaLabel: "Paint blur annotation", icon: <EffectIcon kind="blur" /> },
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
  toolbarRef,
  position,
  width,
  activeAnnotationTool,
  annotationStyle,
  annotationColorPresets,
  fontFamilies = [],
  selectedAnnotationType,
  uiScale = 1,
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
  onClear,
  onCancel,
  onBeginScrollingCapture,
  onRunOcr,
  onCopy,
  onSave,
  onQuickSave,
  onPin,
  onFavorite,
}: CaptureEditorToolbarProps) {
  const primaryModifier = /Mac|iPhone|iPad|iPod/i.test(navigator.platform)
    ? 'Cmd'
    : 'Ctrl';
  const [openPicker, setOpenPicker] = useState<
    "shape" | "arrow" | "redaction" | "color" | "properties" | null
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
  const roundableTypes = ['rectangle', 'region-highlight', 'spotlight'];
  const hasCornerControl = roundableTypes.includes(activeAnnotationTool ?? '') || roundableTypes.includes(selectedAnnotationType ?? '');
  const hasBlurControl = activeAnnotationTool === 'blur' || selectedAnnotationType === 'blur';
  const sizeValue = isTextSizingActive ? textFontSize : annotationStyle.strokeWidth;
  const sizeLabel = isTextSizingActive ? 'Text font size' : 'Annotation stroke width';

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
  const primaryRedactionTool = REDACTION_TOOLS.find(
    (button) => button.tool === activeAnnotationTool,
  ) ?? REDACTION_TOOLS[0];
  const togglePicker = (picker: "shape" | "arrow" | "redaction" | "properties") => {
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

  const quickPropertiesAbove = position.y + 92 * uiScale > window.innerHeight;
  const toolLabel = activeAnnotationTool ? ({ rectangle: '矩形', ellipse: '椭圆', arrow: '箭头', line: '直线', pen: '画笔', highlight: '荧光笔', 'region-highlight': '区域高亮', blur: '模糊', spotlight: '聚光灯', mosaic: '马赛克', text: '文字', eraser: '橡皮擦' })[activeAnnotationTool] : '标注';

  return (
    <div
      ref={toolbarRef}
      data-capture-toolbar="true"
      className={getCaptureEditorToolbarClassName()}
      style={{
        left: `${position.x}px`,
        top: `${position.y}px`,
        width: `${width / uiScale}px`,
        transform: `scale(${uiScale})`,
        transformOrigin: 'top left',
        zIndex: 2,
      }}
      onPointerDown={(event) => {
        event.stopPropagation();
        const target = event.target as HTMLElement;
        if (textDraftActive && target.tagName !== "INPUT" && target.tagName !== "SELECT") {
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
      <ToolGroupButton
        label="Mosaic / Blur"
        groupIcon={primaryRedactionTool.icon}
        primaryTool={primaryRedactionTool}
        tools={REDACTION_TOOLS}
        activeTool={activeAnnotationTool}
        isOpen={openPicker === "redaction"}
        disabled={isRenderingOutput}
        onActivate={selectPickerTool}
        onToggle={() => togglePicker("redaction")}
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
        title={`Undo (${primaryModifier}+Z)`}
        tooltipPlacement="bottom"
        aria-label="Undo annotation"
        onClick={onUndo}
      >
        <UndoIcon />
      </IconActionButton>
      <IconActionButton
        className={getCaptureEditorIconButtonClassName(false)}
        disabled={isRenderingOutput || !canRedo}
        title={`Redo (${primaryModifier}+Y)`}
        tooltipPlacement="bottom"
        aria-label="Redo annotation"
        onClick={onRedo}
      >
        <RedoIcon />
      </IconActionButton>
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
      {(isTextSizingActive || hasCornerControl || hasBlurControl) && (
        <IconActionButton
          className={getCaptureEditorIconButtonClassName(openPicker === "properties")}
          disabled={isRenderingOutput}
          title="Annotation properties"
          aria-label="Annotation properties"
          aria-expanded={openPicker === "properties"}
          onClick={() => togglePicker("properties")}
        >
          ···
        </IconActionButton>
      )}
      {openPicker === "properties" && (isTextSizingActive || hasCornerControl || hasBlurControl) && (
        <div
          className="absolute left-0 flex w-full flex-wrap items-center gap-2 rounded-md border border-line bg-surface px-3 py-2 text-xs text-ink shadow-lg"
          style={quickPropertiesAbove ? { bottom: 'calc(100% + 56px)' } : { top: 'calc(100% + 56px)' }}
          role="group"
          aria-label="Annotation properties"
        >
          {isTextSizingActive && <>
            <select
              className="h-7 max-w-[220px] rounded border border-line bg-surface px-2 text-xs"
              aria-label="Annotation font family"
              value={annotationStyle.fontFamily ?? ''}
              onChange={(event) => onApplyAnnotationStyle({ ...annotationStyle, fontFamily: event.currentTarget.value }, textFontSize)}
            >
              <option value="">默认字体</option>
              {fontFamilies.map((family) => <option key={family} value={family}>{family}</option>)}
            </select>
            <button type="button" className={`h-7 w-7 rounded border border-line font-bold hover:bg-raised ${annotationStyle.bold ? 'bg-accent-soft text-accent-text' : ''}`} aria-label="Bold annotation text" aria-pressed={annotationStyle.bold ?? false} onClick={() => onApplyAnnotationStyle({ ...annotationStyle, bold: !annotationStyle.bold }, textFontSize)}>B</button>
            <button type="button" className={`h-7 w-7 rounded border border-line italic hover:bg-raised ${annotationStyle.italic ? 'bg-accent-soft text-accent-text' : ''}`} aria-label="Italic annotation text" aria-pressed={annotationStyle.italic ?? false} onClick={() => onApplyAnnotationStyle({ ...annotationStyle, italic: !annotationStyle.italic }, textFontSize)}>I</button>
            <div className="flex items-center gap-1">
              <button type="button" className="h-7 w-7 rounded border border-line hover:bg-raised" aria-label="Decrease text font size" onClick={() => { const size = nextTextFontSize(textFontSize, 'decrease', 2); onApplyAnnotationStyle(annotationStyle, size); if (textDraftActive) onTextDraftFontSizeChange(size); }}>−</button>
              <span className="min-w-7 text-center tabular-nums">{textFontSize}</span>
              <button type="button" className="h-7 w-7 rounded border border-line hover:bg-raised" aria-label="Increase text font size" onClick={() => { const size = nextTextFontSize(textFontSize, 'increase', 2); onApplyAnnotationStyle(annotationStyle, size); if (textDraftActive) onTextDraftFontSizeChange(size); }}>+</button>
            </div>
          </>}
          {hasBlurControl && <label className="flex items-center gap-2">模糊强度
            <input type="range" min={MIN_BLUR_RADIUS} max={MAX_BLUR_RADIUS} step={1} value={annotationStyle.blurRadius ?? DEFAULT_BLUR_RADIUS} aria-label="Annotation blur radius" onInput={(event) => onApplyAnnotationStyle({ ...annotationStyle, blurRadius: Number(event.currentTarget.value) }, textFontSize)} />
            <span>{annotationStyle.blurRadius ?? DEFAULT_BLUR_RADIUS}</span>
          </label>}
          {hasCornerControl && <label className="flex items-center gap-2">圆角
            <input type="range" min={0} max={100} step={1} value={annotationStyle.cornerRadius ?? 0} aria-label="Annotation corner radius" onInput={(event) => onApplyAnnotationStyle({ ...annotationStyle, cornerRadius: Number(event.currentTarget.value) }, textFontSize)} />
            <span>{annotationStyle.cornerRadius ?? 0}</span>
          </label>}
        </div>
      )}
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
      <button type="button" className={getCaptureEditorCommandButtonClassName()} disabled={isRenderingOutput} title="长截图" aria-label="Start scrolling capture" onClick={() => { void onBeginScrollingCapture(); }}>长截图</button>
      <button type="button" className={getCaptureEditorCommandButtonClassName()} disabled={isRenderingOutput || !canUndo} title="清空标注（Ctrl/Cmd+Alt+Backspace）" aria-label="Clear annotations" onClick={onClear}>清空</button>
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
        title={`贴图（F3 / ${primaryModifier}+T）`}
        aria-label="Pin capture"
        onClick={() => void onPin()}
      >
        <PinIcon />
      </button>
      <button
        type="button"
        className={getCaptureEditorCommandButtonClassName("icon")}
        disabled={isRenderingOutput}
        title={`Save (${primaryModifier}+S, Shift-click for quick save)`}
        aria-label="Save selection"
        onClick={handleSaveClick}
      >
        <SaveIcon />
      </button>
      <button
        type="button"
        className={getCaptureEditorCommandButtonClassName("primary")}
        disabled={isRenderingOutput}
        title={`Copy (${primaryModifier}+C)`}
        aria-label="Copy selection"
        onClick={() => {
          void onCopy();
        }}
      >
        <CopyIcon />
      </button>
      <div className="capture-quick-properties" role="group" aria-label="标注快捷属性" style={quickPropertiesAbove ? { bottom: 'calc(100% + 8px)' } : { top: 'calc(100% + 8px)' }}>
        <span className="font-semibold text-accent">{toolLabel}</span>
        <div className="capture-quick-colors">
          {visibleColorPresets.slice(0, 5).map((color, index) => <button key={index} type="button" className="capture-quick-color" style={{ backgroundColor: annotationColorToCss(color) }} aria-label={`选择颜色 ${annotationColorToHex(color)}`} aria-pressed={annotationColorsEqual(color, annotationStyle.color)} disabled={isRenderingOutput} onClick={() => applyColor(color)} />)}
        </div>
      <div className="relative flex h-7 shrink-0">
        <IconActionButton
          className="h-7 shrink-0 rounded-md disabled:cursor-not-allowed disabled:opacity-40"
          disabled={isRenderingOutput}
          title="Annotation color"
          tooltipPlacement="bottom"
          aria-label="Annotation color"
          aria-haspopup="dialog"
          aria-expanded={openPicker === "color"}
          onClick={openColorPicker}
        >
          <span className="capture-quick-palette"><span />调色板</span>
        </IconActionButton>
        {openPicker === "color" && (
          <ColorPickerPopover
            colors={visibleColorPresets}
            draftColor={colorDraft}
            selectedIndex={selectedColorPresetIndex}
            error={colorPresetError}
            opensUpward={quickPropertiesAbove || position.y + 380 * uiScale > window.innerHeight}
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
      <div className="flex shrink-0 items-center gap-1.5 pl-1 pr-0.5">
        <span>{isTextSizingActive ? '字号' : '粗细'}</span>
        <input
          className="h-7 w-[86px] accent-accent disabled:opacity-40"
          type="range"
          min={
            isTextSizingActive ? Math.min(MIN_TEXT_FONT_SIZE, textFontSize) : MIN_ANNOTATION_STROKE_WIDTH
          }
          max={
            isTextSizingActive ? Math.max(MAX_TEXT_FONT_SIZE, textFontSize) : MAX_ANNOTATION_STROKE_WIDTH
          }
          step={1}
          value={sizeValue}
          disabled={isRenderingOutput}
          title={
            sizeLabel
          }
          aria-label={
            sizeLabel
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
        <span className="w-7 text-right font-mono text-hint tabular-nums text-muted" title={sizeLabel}>
          {sizeValue}{isTextSizingActive ? '' : 'px'}
        </span>
      </div>
      <button
        type="button"
        className="settings-action"
        disabled={isRenderingOutput || !isFillModeActive}
        title="Fill shape"
        aria-label="Fill shape"
        aria-pressed={annotationStyle.filled}
        onClick={() => onApplyAnnotationStyle({ ...annotationStyle, filled: !annotationStyle.filled }, textFontSize)}
      >{annotationStyle.filled ? '填充模式' : '边框模式'}</button>

      </div>
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
      className={`absolute left-1/2 z-20 w-[248px] -translate-x-1/2 rounded-md border border-line bg-surface p-3 text-ink shadow-float ${
        opensUpward ? "bottom-[calc(100%+3px)]" : "top-[calc(100%+3px)]"
      }`}
      role="dialog"
      aria-label="Annotation colors"
    >
      <div className="mb-2.5 flex items-center justify-between gap-3">
        <span className="text-label font-semibold text-ink">颜色</span>
        <button
          type="button"
          className="flex h-5 w-5 items-center justify-center rounded text-title font-normal text-faint hover:bg-raised hover:text-ink"
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
                className={`h-6 w-6 rounded-sm border border-black/10 shadow-sm transition-transform hover:scale-110 focus:outline-none ${
                  selectedIndex === index
                    ? "ring-2 ring-accent ring-offset-2"
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
        <div className="rounded-md border border-dashed border-line px-3 py-4 text-center text-hint font-normal text-faint">
          暂无预设颜色
        </div>
      )}

      <div className="relative mt-3 h-9">
        <button
          type="button"
          className="flex h-full w-full items-center gap-2 rounded-md border border-line bg-sunken px-2.5 text-left text-hint font-semibold text-ink hover:bg-raised"
          aria-label="Open system color palette"
          aria-haspopup="dialog"
          onClick={() => systemPaletteInputRef.current?.click()}
        >
          <span
            className="h-5 w-5 rounded-full border border-surface shadow-sm"
            style={{
              background:
                "conic-gradient(#ff4d4f, #faad14, #52c41a, #1890ff, #722ed1, #ff4d4f)",
            }}
            aria-hidden="true"
          />
          <span className="flex-1">系统调色板</span>
          <span className="font-mono text-hint text-faint">
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

      <div className="mt-2.5 grid grid-cols-3 gap-1.5 border-t border-line pt-2.5">
        <button
          type="button"
          className="h-7 rounded-md border border-line bg-surface text-hint font-semibold text-muted hover:bg-sunken disabled:cursor-not-allowed disabled:opacity-35"
          disabled={!canAdd}
          aria-label="Add preset color"
          onClick={onAdd}
        >
          新增
        </button>
        <button
          type="button"
          className="h-7 rounded-md border border-line bg-surface text-hint font-semibold text-muted hover:bg-sunken disabled:cursor-not-allowed disabled:opacity-35"
          disabled={!canReplace}
          aria-label="Replace selected preset"
          onClick={onReplace}
        >
          修改
        </button>
        <button
          type="button"
          className="h-7 rounded-md border border-red-200 bg-surface text-hint font-semibold text-red-600 hover:bg-red-50 disabled:cursor-not-allowed disabled:opacity-35"
          disabled={selectedIndex === null}
          aria-label="Delete selected preset"
          onClick={onDelete}
        >
          删除
        </button>
      </div>
      {error && (
        <p className="mt-2 text-hint text-red-600" role="alert">
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
      className="absolute left-0 top-[calc(100%+6px)] z-10 flex min-w-max gap-1 rounded-md border border-line bg-surface p-1 shadow-float"
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

function EffectIcon({ kind }: { kind: 'highlight' | 'blur' }) {
  return <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.6" aria-hidden="true">
    {kind === 'highlight' ? <><path d="m6 14 8-8 4 4-8 8-4-4Z" /><path d="m4 20 3-4 3 3-4 3" /></> : <rect x="4" y="5" width="16" height="14" rx="3" />}
    {kind === 'blur' && <path d="M7 8h10M7 12h10M7 16h10M9 7v10M15 7v10" opacity="0.5" />}
  </svg>;
}
