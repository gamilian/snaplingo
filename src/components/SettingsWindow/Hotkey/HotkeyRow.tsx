import { HotkeyDisplay } from './HotkeyDisplay';

interface HotkeyRowProps {
  label: string;
  value: string;
  description?: string;
  onRecord?: () => void;
  onClear?: () => void;
}

export function HotkeyRow({ label, value, description, onRecord, onClear }: HotkeyRowProps) {
  return (
    <div className="flex items-center justify-between p-4 hover:bg-gray-50 transition-colors">
      <div className="flex-1">
        <div className="text-gray-700 font-medium">{label}</div>
        {description && <div className="text-xs text-gray-500 mt-1">{description}</div>}
      </div>
      <div className="flex items-center space-x-3">
        <HotkeyDisplay value={value} onClick={onRecord} />
        {value !== "未设置" && (
          <button
            onClick={onClear}
            className="w-6 h-6 flex items-center justify-center text-gray-400 hover:text-gray-600 hover:bg-gray-100 rounded transition-colors"
            title="清除快捷键"
          >
            ✕
          </button>
        )}
      </div>
    </div>
  );
}
