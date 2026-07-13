import { useEffect, useRef, useState } from 'react';
import { CustomSelect } from '../../../components/common/CustomSelect';
import { CustomRange } from '../../../components/common/CustomRange';
import { useSettingsConfigStore } from '../../../stores/settingsConfigStore';
import {
  addAnnotationColorPreset,
  annotationColorFromHex,
  annotationColorToHex,
  annotationColorsEqual,
  removeAnnotationColorPreset,
  replaceAnnotationColorPreset,
} from '../../CaptureWorkspace/annotationColorPresets';
import {
  ANNOTATION_COLORS,
  annotationColorToCss,
  type AnnotationColor,
} from '../../CaptureWorkspace/annotationStyle';

export function EditorPage() {
  const [fontSize, setFontSize] = useState('12');
  const [lineWidth, setLineWidth] = useState(3);
  const [pinOpacity, setPinOpacity] = useState(100);
  const [capturedScreenshot, setCapturedScreenshot] = useState<string | null>(null);

  return (
    <div className="max-w-4xl space-y-8">
      <div>
        <h2 className="text-3xl font-bold text-gray-900 mb-2">编辑器</h2>
        <p className="text-gray-600">配置截图编辑工具的默认值</p>
      </div>

      {capturedScreenshot && (
        <div className="bg-white rounded-lg border border-gray-200 p-6 space-y-4">
          <div className="flex items-center justify-between">
            <h3 className="text-lg font-semibold text-gray-800">最近截图</h3>
            <button
              onClick={() => setCapturedScreenshot(null)}
              className="px-3 py-1.5 text-sm text-gray-600 hover:text-gray-900 hover:bg-gray-100 rounded-lg transition-colors"
            >
              清除
            </button>
          </div>
          <div className="overflow-auto rounded-lg border border-gray-200 bg-gray-100 max-h-[520px]">
            <img
              src={capturedScreenshot}
              alt="最近截图"
              className="block max-w-full h-auto mx-auto"
            />
          </div>
        </div>
      )}

      {/* 默认工具 */}
      <div className="bg-white rounded-lg border border-gray-200 p-6 space-y-6">
        <h3 className="text-lg font-semibold text-gray-800">默认工具</h3>

        <AnnotationColorSettings />

        {/* 默认线条粗细 */}
        <div className="pt-6 border-t border-gray-100">
          <label className="block font-medium text-gray-700 mb-2">
            默认线条粗细：<span className="text-primary-600">{lineWidth}px</span>
          </label>
          <CustomRange
            value={lineWidth}
            onChange={setLineWidth}
            min={1}
            max={10}
            step={1}
          />
        </div>

        {/* 默认字体大小 */}
        <div className="pt-6 border-t border-gray-100">
          <label className="block font-medium text-gray-700 mb-2">默认字体大小</label>
          <CustomSelect
            options={[
              { value: '12', label: '12px（小）' },
              { value: '16', label: '16px（中）' },
              { value: '20', label: '20px（大）' },
              { value: '24', label: '24px（特大）' },
            ]}
            value={fontSize}
            onChange={setFontSize}
          />
          <p className="text-sm text-gray-500 mt-2">文字标注的默认字体大小</p>
        </div>
      </div>

      {/* 行为设置 */}
      <div className="bg-white rounded-lg border border-gray-200 p-6 space-y-6">
        <h3 className="text-lg font-semibold text-gray-800">编辑行为</h3>

        {/* 自动选中工具 */}
        <div className="flex items-center justify-between">
          <div>
            <div className="font-medium text-gray-700">记住上次使用的工具</div>
            <div className="text-sm text-gray-500 mt-1">下次截图时自动选中上次使用的标注工具</div>
          </div>
          <button className="relative w-12 h-6 rounded-full bg-primary-600 transition-colors">
            <span className="absolute top-0.5 left-0.5 w-5 h-5 bg-white rounded-full shadow translate-x-6 transition-transform" />
          </button>
        </div>

        {/* 显示尺寸 */}
        <div className="flex items-center justify-between pt-6 border-t border-gray-100">
          <div>
            <div className="font-medium text-gray-700">显示选区尺寸</div>
            <div className="text-sm text-gray-500 mt-1">截图时显示选区的像素尺寸</div>
          </div>
          <button className="relative w-12 h-6 rounded-full bg-primary-600 transition-colors">
            <span className="absolute top-0.5 left-0.5 w-5 h-5 bg-white rounded-full shadow translate-x-6 transition-transform" />
          </button>
        </div>

        {/* 放大镜 */}
        <div className="flex items-center justify-between pt-6 border-t border-gray-100">
          <div>
            <div className="font-medium text-gray-700">显示放大镜</div>
            <div className="text-sm text-gray-500 mt-1">截图时显示像素级放大镜，便于精确选取</div>
          </div>
          <button className="relative w-12 h-6 rounded-full bg-gray-300 transition-colors">
            <span className="absolute top-0.5 left-0.5 w-5 h-5 bg-white rounded-full shadow transition-transform" />
          </button>
        </div>
      </div>

      {/* 贴图设置 */}
      <div className="bg-white rounded-lg border border-gray-200 p-6 space-y-6">
        <h3 className="text-lg font-semibold text-gray-800">贴图设置</h3>

        {/* 默认透明度 */}
        <div>
          <label className="block font-medium text-gray-700 mb-2">
            贴图默认透明度：<span className="text-primary-600">{pinOpacity}%</span>
          </label>
          <CustomRange
            value={pinOpacity}
            onChange={setPinOpacity}
            min={20}
            max={100}
            step={1}
          />
        </div>

        {/* 阴影 */}
        <div className="pt-6 border-t border-gray-100 flex items-center justify-between">
          <div>
            <div className="font-medium text-gray-700">贴图显示阴影</div>
            <div className="text-sm text-gray-500 mt-1">为贴图窗口添加投影效果</div>
          </div>
          <button className="relative w-12 h-6 rounded-full bg-primary-600 transition-colors">
            <span className="absolute top-0.5 left-0.5 w-5 h-5 bg-white rounded-full shadow translate-x-6 transition-transform" />
          </button>
        </div>
      </div>
    </div>
  );
}

function AnnotationColorSettings() {
  const storedColors = useSettingsConfigStore(
    (state) => state.screenshot?.annotationColors,
  );
  const updateAnnotationColors = useSettingsConfigStore(
    (state) => state.updateAnnotationColors,
  );
  const colors = storedColors ?? ANNOTATION_COLORS;
  const [draftColor, setDraftColor] = useState(() =>
    annotationColorToHex(colors[0] ?? ANNOTATION_COLORS[0]),
  );
  const [selectedIndex, setSelectedIndex] = useState<number | null>(0);
  const selectedIndexRef = useRef<number | null>(selectedIndex);
  const [isSaving, setIsSaving] = useState(false);
  const [saveError, setSaveError] = useState<string | null>(null);
  const systemPaletteInputRef = useRef<HTMLInputElement>(null);
  selectedIndexRef.current = selectedIndex;
  const parsedDraftColor = annotationColorFromHex(draftColor);
  const selectedColor =
    selectedIndex === null ? null : (colors[selectedIndex] ?? null);
  const duplicateIndex = parsedDraftColor
    ? colors.findIndex((color) =>
        annotationColorsEqual(color, parsedDraftColor),
      )
    : -1;
  const canAdd = parsedDraftColor !== null && duplicateIndex < 0;
  const canReplace =
    selectedIndex !== null &&
    parsedDraftColor !== null &&
    selectedColor !== null &&
    !annotationColorsEqual(selectedColor, parsedDraftColor) &&
    (duplicateIndex < 0 || duplicateIndex === selectedIndex);

  useEffect(() => {
    if (colors.length === 0) {
      setSelectedIndex(null);
      return;
    }

    const currentIndex = selectedIndexRef.current;
    const nextIndex =
      currentIndex === null ? 0 : Math.min(currentIndex, colors.length - 1);
    setSelectedIndex(nextIndex);
    setDraftColor(annotationColorToHex(colors[nextIndex]));
  }, [colors]);

  const persistColors = async (nextColors: AnnotationColor[]) => {
    setIsSaving(true);
    setSaveError(null);
    try {
      await updateAnnotationColors(nextColors);
      return true;
    } catch {
      setSaveError('颜色预设保存失败，请重试。');
      return false;
    } finally {
      setIsSaving(false);
    }
  };

  const selectColor = (color: AnnotationColor, index: number) => {
    setSelectedIndex(index);
    setDraftColor(annotationColorToHex(color));
  };

  const changeDraftColor = (value: string) => {
    setDraftColor(value.toUpperCase());
  };

  const addColor = async () => {
    if (!parsedDraftColor) return;

    const nextColors = addAnnotationColorPreset(colors, parsedDraftColor);
    if (nextColors.length === colors.length) return;

    if (await persistColors(nextColors)) {
      setSelectedIndex(nextColors.length - 1);
      setDraftColor(annotationColorToHex(parsedDraftColor));
    }
  };

  const replaceColor = async () => {
    if (selectedIndex === null || !parsedDraftColor) return;

    await persistColors(
      replaceAnnotationColorPreset(colors, selectedIndex, parsedDraftColor),
    );
  };

  const deleteColor = async () => {
    if (selectedIndex === null) return;

    const nextColors = removeAnnotationColorPreset(colors, selectedIndex);
    const nextIndex =
      nextColors.length === 0
        ? null
        : Math.min(selectedIndex, nextColors.length - 1);
    if (await persistColors(nextColors)) {
      setSelectedIndex(nextIndex);
      if (nextIndex !== null) {
        setDraftColor(annotationColorToHex(nextColors[nextIndex]));
      }
    }
  };

  return (
    <div>
      <div className="flex items-center justify-between gap-4">
        <div>
          <label className="block font-medium text-gray-700">标注颜色</label>
          <p className="mt-1 text-sm text-gray-500">
            编辑器工具栏会使用这里管理的预设颜色。
          </p>
        </div>
        <div className="relative h-9">
          <button
            type="button"
            className="inline-flex h-full items-center gap-2 rounded-lg border border-gray-200 bg-white px-3 text-sm font-medium text-gray-700 transition-colors hover:bg-gray-50"
            onClick={() => systemPaletteInputRef.current?.click()}
          >
            <span
              className="h-4 w-4 rounded-full border border-white shadow-sm"
              style={{
                background:
                  'conic-gradient(#ff4d4f, #faad14, #52c41a, #1890ff, #722ed1, #ff4d4f)',
              }}
            />
            系统调色板
          </button>
          <input
            ref={systemPaletteInputRef}
            className="pointer-events-none absolute inset-0 h-full w-full opacity-0"
            type="color"
            value={parsedDraftColor ? draftColor : '#000000'}
            tabIndex={-1}
            aria-label="选择系统颜色"
            onInput={(event) => changeDraftColor(event.currentTarget.value)}
          />
        </div>
      </div>

      <div className="mt-4 grid grid-cols-8 gap-2" aria-label="标注颜色预设">
        {colors.map((color, index) => {
          const hexColor = annotationColorToHex(color);
          return (
            <button
              key={`${hexColor}-${index}`}
              type="button"
              className={`h-9 w-9 rounded-lg border border-black/10 shadow-sm transition-transform hover:scale-105 focus:outline-none ${
                selectedIndex === index
                  ? 'ring-2 ring-primary-500 ring-offset-2'
                  : ''
              }`}
              style={{ backgroundColor: annotationColorToCss(color) }}
              title={hexColor}
              aria-label={`选择预设颜色 ${hexColor}`}
              onClick={() => selectColor(color, index)}
            />
          );
        })}
      </div>

      <div className="mt-4 flex flex-wrap items-center gap-3 rounded-lg border border-gray-100 bg-gray-50 p-3">
        <span
          className="h-9 w-9 rounded-lg border border-black/10 shadow-sm"
          style={{ backgroundColor: parsedDraftColor ? draftColor : '#ffffff' }}
          aria-hidden="true"
        />
        <input
          className="h-9 w-28 rounded-lg border border-gray-200 bg-white px-2 font-mono text-sm uppercase text-gray-700 outline-none transition-colors focus:border-primary-500 focus:ring-2 focus:ring-primary-100"
          value={draftColor}
          aria-label="颜色十六进制值"
          onChange={(event) => changeDraftColor(event.currentTarget.value)}
        />
        <button
          type="button"
          className="h-9 rounded-lg border border-gray-200 bg-white px-3 text-sm font-medium text-gray-700 transition-colors hover:bg-gray-50 disabled:cursor-not-allowed disabled:opacity-40"
          disabled={isSaving || !canAdd}
          onClick={() => void addColor()}
        >
          新增
        </button>
        <button
          type="button"
          className="h-9 rounded-lg border border-gray-200 bg-white px-3 text-sm font-medium text-gray-700 transition-colors hover:bg-gray-50 disabled:cursor-not-allowed disabled:opacity-40"
          disabled={isSaving || !canReplace}
          onClick={() => void replaceColor()}
        >
          修改
        </button>
        <button
          type="button"
          className="h-9 rounded-lg border border-red-100 bg-white px-3 text-sm font-medium text-red-600 transition-colors hover:bg-red-50 disabled:cursor-not-allowed disabled:opacity-40"
          disabled={isSaving || selectedIndex === null}
          onClick={() => void deleteColor()}
        >
          删除
        </button>
      </div>
      {saveError && (
        <p className="mt-2 text-sm text-red-600" role="alert">
          {saveError}
        </p>
      )}
    </div>
  );
}
