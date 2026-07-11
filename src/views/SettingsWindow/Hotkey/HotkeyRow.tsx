import { HotkeyDisplay } from './HotkeyDisplay';
import IconActionButton from '../../../components/common/IconActionButton';

interface HotkeyRowProps {
  label: string;
  value: string;
  description?: string;
  onRecord?: () => void;
  onClear?: () => void;
  onReset?: () => void;
  isRecording?: boolean;
  defaultValue?: string;
}

export function HotkeyRow({
  label,
  value,
  description,
  onRecord,
  onClear,
  onReset,
  isRecording = false,
  defaultValue
}: HotkeyRowProps) {
  const hasValue = value !== "未设置";
  const isModified = defaultValue && value !== defaultValue;

  return (
    <div className="flex items-center justify-between p-4 hover:bg-gray-50 transition-colors">
      <div className="flex-1">
        <div className="text-gray-700 font-medium">{label}</div>
        {description && <div className="text-xs text-gray-500 mt-1">{description}</div>}
      </div>
      <div className="flex items-center space-x-3">
        <HotkeyDisplay value={value} onClick={onRecord} isRecording={isRecording} />

        {/* 操作按钮组 - 始终显示 */}
        <div className="flex items-center space-x-1">
          {/* 恢复默认按钮 */}
          <IconActionButton
            onClick={onReset}
            disabled={!isModified}
            className={`
              w-7 h-7 flex items-center justify-center rounded transition-all duration-150
              ${isModified
                ? 'text-gray-400 hover:text-primary-600 hover:bg-primary-50 cursor-pointer'
                : 'text-gray-300 cursor-not-allowed'
              }
            `}
            title={isModified ? "恢复默认值" : "已是默认值"}
          >
            <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
            </svg>
          </IconActionButton>

          {/* 删除按钮 */}
          <IconActionButton
            onClick={onClear}
            disabled={!hasValue}
            className={`
              w-7 h-7 flex items-center justify-center rounded transition-all duration-150
              ${hasValue
                ? 'text-gray-400 hover:text-red-500 hover:bg-red-50 cursor-pointer'
                : 'text-gray-300 cursor-not-allowed'
              }
            `}
            title={hasValue ? "清除快捷键" : "未设置快捷键"}
          >
            <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
            </svg>
          </IconActionButton>
        </div>
      </div>
    </div>
  );
}
