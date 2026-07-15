import type { ReactNode } from 'react';
import type { AnnotationColorPreset, ScreenshotSettings } from '../../../application/settings/ports';
import { CustomRange } from '../../../components/common/CustomRange';
import { CustomSelect } from '../../../components/common/CustomSelect';
import { useSettingsConfigStore } from '../../../stores/settingsConfigStore';
import { FeatureHotkeysSection } from '../Hotkey/FeatureHotkeysSection';
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
  [255, 255, 255, 72],
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
    return <div className="p-12 text-sm text-gray-500">设置加载中...</div>;
  }

  const updateScreenshot = (input: Partial<ScreenshotSettings>) => {
    void updateScreenshotSettings(input);
  };
  const selectionMaskColor = screenshot.selectionMaskColor ?? MASK_COLORS[0];

  const outputSettings = (
    <>
      <SettingsGroup title="文件">
        <SettingRow label="默认保存路径" description="截图文件保存到此文件夹">
          <div className="flex items-center gap-2">
            <input
              type="text"
              key={screenshot.savePath}
              defaultValue={screenshot.savePath}
              onBlur={(event) =>
                updateScreenshot({ savePath: event.currentTarget.value })
              }
              className="h-9 w-[260px] rounded-lg border border-gray-200 bg-white px-3 text-xs text-gray-700 outline-none focus:border-primary-400 focus:ring-2 focus:ring-primary-100"
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
              className="h-9 rounded-lg border border-gray-200 bg-white px-3 text-xs font-medium text-gray-600 hover:bg-gray-50"
            >
              浏览
            </button>
          </div>
        </SettingRow>
        <SettingRow label="图片格式">
          <div className="w-[230px]">
            <CustomSelect
              value={screenshot.format}
              options={[
                { value: 'png', label: 'PNG' },
                { value: 'jpg', label: 'JPG' },
                { value: 'webp', label: 'WebP' },
              ]}
              onChange={(format) =>
                updateScreenshot({ format: format as ScreenshotSettings['format'] })
              }
            />
          </div>
        </SettingRow>
        <SettingRow label="图片质量" description="仅对 JPG 和 WebP 格式生效">
          <RangeValue value={`${screenshot.quality}%`}>
            <CustomRange
              value={screenshot.quality}
              min={50}
              max={100}
              step={1}
              onChange={(quality) => updateScreenshot({ quality })}
            />
          </RangeValue>
        </SettingRow>
        <SettingRow label="文件命名规则">
          <div className="w-[230px]">
            <CustomSelect
              value={screenshot.namingRule}
              options={[
                { value: 'timestamp', label: '时间戳' },
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
              className="h-9 w-[230px] rounded-lg border border-gray-200 px-3 text-xs outline-none focus:border-primary-400"
            />
          </SettingRow>
        )}
      </SettingsGroup>
      <SettingsGroup title="完成后">
        <SettingRow label="截图后自动复制" description="截图完成后复制到剪贴板">
          <SettingsToggle
            label="截图后自动复制"
            checked={screenshot.autoCopy}
            onChange={(autoCopy) => updateScreenshot({ autoCopy })}
          />
        </SettingRow>
      </SettingsGroup>
    </>
  );

  const captureInterfaceSettings = (
    <>
      <SettingsGroup title="选区外观">
        <SettingRow label="边框宽度" description="截图选区边框的显示宽度">
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
        <SettingRow label="遮罩颜色" description="选区外区域使用的遮罩颜色">
          <div className="flex items-center gap-2.5">
            {MASK_COLORS.map((color) => {
              const selected = colorsEqual(selectionMaskColor, color);
              return (
                <button
                  key={color.join('-')}
                  type="button"
                  aria-label={`遮罩颜色 ${color.slice(0, 3).join(',')}`}
                  onClick={() => updateScreenshot({ selectionMaskColor: color })}
                  className={`h-8 w-8 rounded-lg border-[3px] border-white shadow-sm ${
                    selected ? 'ring-2 ring-primary-500' : 'ring-1 ring-gray-200'
                  }`}
                  style={{ backgroundColor: rgba(color) }}
                />
              );
            })}
            <label className="relative ml-1 flex h-9 cursor-pointer items-center gap-2 rounded-lg border border-gray-200 bg-white px-3 text-xs font-medium text-gray-600 shadow-sm hover:bg-gray-50">
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
        <SettingRow label="遮罩透明度" description="调整选区外遮罩的透明程度">
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
      <SettingsGroup title="选区辅助">
        <SettingRow label="显示选区尺寸" description="选取时显示宽度和高度">
          <SettingsToggle
            label="显示选区尺寸"
            checked={screenshot.showSelectionSize}
            onChange={(showSelectionSize) => updateScreenshot({ showSelectionSize })}
          />
        </SettingRow>
        <SettingRow
          label="显示放大镜"
          description="选择截图区域时跟随鼠标显示像素、坐标和颜色值"
        >
          <SettingsToggle
            label="显示放大镜"
            checked={screenshot.showMagnifier}
            onChange={(showMagnifier) => updateScreenshot({ showMagnifier })}
          />
        </SettingRow>
      </SettingsGroup>
      <SettingsGroup title="编辑器与贴图">
        <SettingRow label="记住上次使用的工具" description="下次截图时恢复上次使用的标注工具">
          <SettingsToggle
            label="记住上次使用的工具"
            checked={screenshot.rememberLastTool}
            onChange={(rememberLastTool) => updateScreenshot({ rememberLastTool })}
          />
        </SettingRow>
        <SettingRow label="贴图默认透明度">
          <RangeValue value={`${screenshot.pinOpacity}%`}>
            <CustomRange
              value={screenshot.pinOpacity}
              min={20}
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
      </SettingsGroup>
    </>
  );

  return (
    <SettingsScrollPage
      title="截图"
      description="截图快捷键、文件输出与选区界面"
      sections={[
        {
          id: 'hotkeys',
          label: '快捷键',
          description: '截图与贴图的全局快捷键',
          content: (
            <FeatureHotkeysSection
              category="screenshot"
              actions={SCREENSHOT_HOTKEYS}
            />
          ),
        },
        {
          id: 'output',
          label: '保存与输出',
          description: '保存位置、图片格式和完成后的动作',
          content: outputSettings,
        },
        {
          id: 'capture-interface',
          label: '截图界面',
          description: '配置选区外观、辅助信息与贴图显示',
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
      <span className="min-w-[38px] text-right text-[11px] font-semibold text-primary-600">
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
