import type { ReactNode } from 'react';
import { AnnotationShortcutSettings } from './AnnotationShortcutSettings';
import { PinBorderShortcutSettings } from './PinBorderShortcutSettings';
import type { AnnotationColorPreset, ScreenshotSettings } from '../../../application/settings/ports';
import { CustomRange } from '../../../components/common/CustomRange';
import { CustomSelect } from '../../../components/common/CustomSelect';
import { useSettingsConfigStore } from '../../../stores/settingsConfigStore';
import { FeatureHotkeysSection, HotkeyToolbar } from '../Hotkey/FeatureHotkeysSection';
import { SettingRow, SettingsGroup, SettingsToggle } from '../SettingsControls';
import { SettingsScrollPage } from '../SettingsScrollPage';
import { useSettingsRuntime } from '../runtimeContext';
import {
  maskColorHex,
  maskColorOpacity,
  maskColorWithHex,
  maskColorWithOpacity,
} from './maskColor';

const MASK_COLORS: AnnotationColorPreset[] = [
  [0, 0, 0, 46],
  [32, 36, 44, 72],
  [62, 70, 84, 72],
  [20, 55, 92, 64],
  [25, 82, 72, 64],
  [88, 54, 112, 64],
  [112, 68, 44, 64],
  [255, 255, 255, 72],
];

const BORDER_COLORS: AnnotationColorPreset[] = [
  [91, 127, 255, 242],
  [255, 77, 79, 242],
  [40, 167, 69, 242],
  [250, 219, 20, 242],
  [255, 255, 255, 242],
  [0, 0, 0, 242],
];

const SCREENSHOT_HOTKEYS = [
  { key: 'screenshot', label: '截屏' },
  { key: 'screenshot-copy', label: '截屏并自动复制' },
  { key: 'pin', label: '贴图' },
  { key: 'pin-toggle-all', label: '隐藏 / 显示所有贴图' },
  { key: 'pin-switch-group', label: '切换到另一贴图组' },
];

export function ScreenshotSettingsPage() {
  const runtime = useSettingsRuntime();
  const screenshot = useSettingsConfigStore((state) => state.screenshot);
  const updateScreenshotSettings = useSettingsConfigStore(
    (state) => state.updateScreenshotSettings,
  );
  if (!screenshot) {
    return <div className="p-12 text-sm text-muted">设置加载中...</div>;
  }

  const updateScreenshot = (input: Partial<ScreenshotSettings>) => {
    void updateScreenshotSettings(input);
  };
  const selectionBorderColor =
    screenshot.selectionBorderColor ?? BORDER_COLORS[0];
  const selectionMaskColor = screenshot.selectionMaskColor ?? MASK_COLORS[0];

  const outputSettings = (
    <>
      <SettingsGroup>
        <SettingRow label="默认保存路径">
          <div className="flex items-center gap-2">
            <input
              type="text"
              key={screenshot.savePath}
              defaultValue={screenshot.savePath}
              onBlur={(event) =>
                updateScreenshot({ savePath: event.currentTarget.value })
              }
              className="h-9 w-[260px] rounded-lg border border-line bg-surface px-3 text-xs text-ink outline-none focus:border-accent focus:ring-2 focus:ring-accent-line"
              placeholder="~/Pictures/SnapLingo"
            />
            <button
              type="button"
              onClick={() => {
                void runtime.window.selectScreenshotDirectory().then((savePath) => {
                  if (savePath) {
                    updateScreenshot({ savePath });
                  }
                });
              }}
              className="h-9 rounded-lg border border-line bg-surface px-3 text-xs font-medium text-muted hover:bg-sunken"
            >
              浏览
            </button>
          </div>
        </SettingRow>
        <SettingRow label="图片格式" description="快捷保存使用此格式；另存为记住上次目录和格式。">
          <div className="w-[230px]">
            <CustomSelect
              value={screenshot.format}
              options={[
                { value: 'png', label: 'PNG（无损）' },
                { value: 'jpg', label: 'JPG' },
                { value: 'webp', label: 'WebP（无损）' },
              ]}
              onChange={(format) =>
                updateScreenshot({ format: format as ScreenshotSettings['format'] })
              }
            />
          </div>
        </SettingRow>
        {screenshot.format === 'jpg' && <SettingRow label="JPEG 图片质量">
          <RangeValue value={`${screenshot.quality}%`}>
            <CustomRange
              value={screenshot.quality}
              min={50}
              max={100}
              step={1}
              onChange={(quality) => updateScreenshot({ quality })}
            />
          </RangeValue>
        </SettingRow>}
        <SettingRow label="文件命名规则">
          <div className="w-[230px]">
            <CustomSelect
              value={screenshot.namingRule}
              options={[
                { value: 'timestamp', label: '日期与时间' },
                { value: 'unix-ms', label: 'Unix 毫秒时间戳' },
                { value: 'date', label: '日期' },
                { value: 'counter', label: '计数器' },
                { value: 'custom', label: '自定义' },
              ]}
              onChange={(namingRule) =>
                updateScreenshot({
                  namingRule: namingRule as ScreenshotSettings['namingRule'],
                })
              }
            />
          </div>
        </SettingRow>
        {screenshot.namingRule === 'custom' && (
          <SettingRow label="自定义文件名前缀">
            <input
              type="text"
              value={screenshot.customFileName}
              onChange={(event) =>
                updateScreenshot({ customFileName: event.target.value })
              }
              className="h-9 w-[230px] rounded-lg border border-line px-3 text-xs outline-none focus:border-accent"
            />
          </SettingRow>
        )}
      </SettingsGroup>
      <SettingsGroup>
        <SettingRow label="截图后自动复制">
          <SettingsToggle
            label="截图后自动复制"
            checked={screenshot.autoCopy}
            onChange={(autoCopy) => updateScreenshot({ autoCopy })}
          />
        </SettingRow>
        <SettingRow label="长截图最大高度" description="单位：像素。到达上限后自动复制并保存 PNG；同时受总像素上限保护。">
          <div className="flex items-center gap-2">
            <input type="number" aria-label="长截图最大高度" min={100} max={100000} step={1000} key={screenshot.longshotMaxHeight ?? 30000} defaultValue={screenshot.longshotMaxHeight ?? 30000}
              onBlur={(event) => { const value = Number(event.currentTarget.value); if (Number.isFinite(value) && value >= 100) updateScreenshot({ longshotMaxHeight: Math.min(100000, Math.round(value)) }); }}
              className="h-9 w-[140px] rounded-lg border border-line bg-surface px-3 text-xs text-ink outline-none focus:border-accent" />
            <span className="text-xs text-muted">px</span>
          </div>
        </SettingRow>
      </SettingsGroup>
    </>
  );

  const captureInterfaceSettings = (
    <>
      <SettingsGroup>
        <SettingRow label="边框宽度">
          <RangeValue value={`${screenshot.selectionBorderWidth ?? 2}px`}>
            <CustomRange
              value={screenshot.selectionBorderWidth ?? 2}
              min={1}
              max={8}
              step={1}
              onChange={(selectionBorderWidth) =>
                updateScreenshot({ selectionBorderWidth })
              }
            />
          </RangeValue>
        </SettingRow>
        <SettingRow label="边框颜色">
          <div className="flex items-center gap-2.5">
            {BORDER_COLORS.map((color) => {
              const selected = colorsEqual(selectionBorderColor, color);
              return (
                <button
                  key={color.join('-')}
                  type="button"
                  aria-label={`边框颜色 ${color.slice(0, 3).join(',')}`}
                  onClick={() => updateScreenshot({ selectionBorderColor: color })}
                  className={`h-8 w-8 rounded-lg border-[3px] border-white shadow-sm ${
                    selected ? 'ring-2 ring-accent' : 'ring-1 ring-gray-200'
                  }`}
                  style={{ backgroundColor: rgba(color) }}
                />
              );
            })}
            <label className="relative ml-1 flex h-9 cursor-pointer items-center gap-2 rounded-lg border border-line bg-surface px-3 text-xs font-medium text-muted shadow-sm hover:bg-sunken">
              <span
                className="h-4 w-4 rounded-full border border-black/10"
                style={{ backgroundColor: maskColorHex(selectionBorderColor) }}
              />
              调色板
              <input
                type="color"
                aria-label="自定义边框颜色"
                value={maskColorHex(selectionBorderColor)}
                onChange={(event) =>
                  updateScreenshot({
                    selectionBorderColor: maskColorWithHex(
                      selectionBorderColor,
                      event.currentTarget.value,
                    ),
                  })
                }
                className="absolute inset-0 cursor-pointer opacity-0"
              />
            </label>
          </div>
        </SettingRow>
        <SettingRow label="遮罩颜色">
          <div className="flex max-w-[520px] flex-wrap items-center justify-end gap-2.5">
            {MASK_COLORS.map((color) => {
              const selected = colorsEqual(selectionMaskColor, color);
              return (
                <button
                  key={color.join('-')}
                  type="button"
                  aria-label={`遮罩颜色 ${color.slice(0, 3).join(',')}`}
                  onClick={() => updateScreenshot({ selectionMaskColor: color })}
                  className={`h-8 w-8 rounded-lg border-[3px] border-white shadow-sm ${
                    selected ? 'ring-2 ring-accent' : 'ring-1 ring-gray-200'
                  }`}
                  style={{ backgroundColor: rgba(color) }}
                />
              );
            })}
            <label className="relative ml-1 flex h-9 cursor-pointer items-center gap-2 rounded-lg border border-line bg-surface px-3 text-xs font-medium text-muted shadow-sm hover:bg-sunken">
              <span
                className="h-4 w-4 rounded-full border border-black/10"
                style={{ backgroundColor: maskColorHex(selectionMaskColor) }}
              />
              调色板
              <input
                type="color"
                aria-label="自定义遮罩颜色"
                value={maskColorHex(selectionMaskColor)}
                onChange={(event) =>
                  updateScreenshot({
                    selectionMaskColor: maskColorWithHex(
                      selectionMaskColor,
                      event.currentTarget.value,
                    ),
                  })
                }
                className="absolute inset-0 cursor-pointer opacity-0"
              />
            </label>
          </div>
        </SettingRow>
        <SettingRow label="遮罩透明度">
          <RangeValue value={`${maskColorOpacity(selectionMaskColor)}%`}>
            <CustomRange
              value={maskColorOpacity(selectionMaskColor)}
              min={0}
              max={90}
              step={1}
              onChange={(opacity) =>
                updateScreenshot({
                  selectionMaskColor: maskColorWithOpacity(
                    selectionMaskColor,
                    opacity,
                  ),
                })
              }
            />
          </RangeValue>
        </SettingRow>
      </SettingsGroup>
      <SettingsGroup>
        <SettingRow label="记住上次标注工具" description="本次运行内沿用上次工具；关闭后每次从矩形开始。">
          <SettingsToggle
            label="记住上次标注工具"
            checked={screenshot.rememberLastTool}
            onChange={(rememberLastTool) => updateScreenshot({ rememberLastTool })}
          />
        </SettingRow>
        <SettingRow label="显示选区尺寸">
          <SettingsToggle
            label="显示选区尺寸"
            checked={screenshot.showSelectionSize}
            onChange={(showSelectionSize) => updateScreenshot({ showSelectionSize })}
          />
        </SettingRow>
        <SettingRow
          label="显示放大镜"
         
        >
          <SettingsToggle
            label="显示放大镜"
            checked={screenshot.showMagnifier}
            onChange={(showMagnifier) => updateScreenshot({ showMagnifier })}
          />
        </SettingRow>
        <SettingRow label="放大镜倍率">
          <RangeValue value={`${screenshot.magnifierZoom ?? 12}×`}>
            <CustomRange
              value={screenshot.magnifierZoom ?? 12}
              min={4}
              max={20}
              step={1}
              onChange={(magnifierZoom) => updateScreenshot({ magnifierZoom })}
            />
          </RangeValue>
        </SettingRow>
      </SettingsGroup>
      <SettingsGroup>
        <SettingRow label="贴图默认透明度">
          <RangeValue value={`${screenshot.pinOpacity}%`}>
            <CustomRange
              value={screenshot.pinOpacity}
              min={5}
              max={100}
              step={1}
              onChange={(pinOpacity) => updateScreenshot({ pinOpacity })}
            />
          </RangeValue>
        </SettingRow>
        <SettingRow label="贴图显示阴影">
          <SettingsToggle
            label="贴图显示阴影"
            checked={screenshot.pinShadow}
            onChange={(pinShadow) => updateScreenshot({ pinShadow })}
          />
        </SettingRow>
        <SettingRow label="贴图初始位置">
          <div className="w-[230px]"><CustomSelect
            value={screenshot.pinPosition ?? 'original'}
            options={[{ value: 'original', label: '截图原位置' }, { value: 'center', label: '桌面中央（级联）' }]}
            onChange={(position) => updateScreenshot({ pinPosition: position as 'original' | 'center' })}
          /></div>
        </SettingRow>
        <SettingRow label="显示贴图边框">
          <SettingsToggle label="显示贴图边框" checked={screenshot.pinBorder ?? false} onChange={(pinBorder) => updateScreenshot({ pinBorder })} />
        </SettingRow>
        <SettingRow label="界面缩放" description="应用于设置、截图工具、长截图控件与光标。">
          <div className="w-[230px]"><CustomSelect
            value={screenshot.captureUiScale ?? 'auto'}
            options={[{ value: 'auto', label: '自动（跟随显示器）' }, ...['100', '125', '150', '175', '200'].map((value) => ({ value, label: `${value}%` }))]}
            onChange={(scale) => updateScreenshot({ captureUiScale: scale as ScreenshotSettings['captureUiScale'] })}
          /></div>
        </SettingRow>
      </SettingsGroup>
    </>
  );

  return (
    <SettingsScrollPage
      title="截图"
     
      sections={[
        {
          id: 'hotkeys',
          label: '快捷键',
          action: (
            <HotkeyToolbar category="screenshot" actions={SCREENSHOT_HOTKEYS} />
          ),
          content: (
            <>
            <FeatureHotkeysSection
              category="screenshot"
              actions={SCREENSHOT_HOTKEYS}
            />
            <AnnotationShortcutSettings value={screenshot.annotationToolShortcuts} onUpdate={(annotationToolShortcuts) => updateScreenshotSettings({ annotationToolShortcuts })} />
            <SettingsGroup><PinBorderShortcutSettings value={screenshot.pinBorderShortcut} onUpdate={(pinBorderShortcut) => updateScreenshotSettings({ pinBorderShortcut })} /></SettingsGroup>
            </>
          ),
        },
        {
          id: 'output',
          label: '保存与输出',
          content: outputSettings,
        },
        {
          id: 'capture-interface',
          label: '截图界面',
          content: captureInterfaceSettings,
        },
      ]}
    />
  );
}

function RangeValue({ children, value }: { children: ReactNode; value: string }) {
  return (
    <div className="flex w-[250px] items-center gap-3">
      <div className="min-w-0 flex-1">{children}</div>
      <span className="min-w-[38px] text-right text-hint font-semibold text-accent-text">
        {value}
      </span>
    </div>
  );
}

function colorsEqual(left: AnnotationColorPreset, right: AnnotationColorPreset) {
  return left.every((channel, index) => channel === right[index]);
}

function rgba(color: AnnotationColorPreset) {
  return `rgba(${color[0]}, ${color[1]}, ${color[2]}, ${color[3] / 255})`;
}
