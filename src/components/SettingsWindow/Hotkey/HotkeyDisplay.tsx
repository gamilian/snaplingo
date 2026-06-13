interface HotkeyDisplayProps {
  value: string; // e.g., "⌥S", "⇧⌥S", "未设置"
  onClick?: () => void;
}

export function HotkeyDisplay({ value, onClick }: HotkeyDisplayProps) {
  const handleClick = () => {
    console.log('[HotkeyDisplay] Clicked! Value:', value);
    onClick?.();
  };

  // 解析快捷键
  const hasShift = value.includes('⇧');
  const hasOption = value.includes('⌥');
  const hasCommand = value.includes('⌘');
  const hasControl = value.includes('⌃');

  // 提取字母键（最后一个非修饰键字符）
  const letterKey = value.replace(/[⇧⌥⌘⌃]/g, '').trim();

  const isUnset = value === "未设置";

  if (isUnset) {
    return (
      <button
        onClick={handleClick}
        className="flex items-center space-x-1 px-4 py-2 bg-white border-2 border-dashed border-gray-300 rounded-lg text-gray-400 text-sm min-w-[280px] justify-center hover:border-gray-400 transition-colors cursor-pointer"
      >
        按下快捷键
      </button>
    );
  }

  return (
    <button
      onClick={handleClick}
      className="flex items-center space-x-1 px-4 py-2 bg-white border border-gray-200 rounded-lg min-w-[280px] justify-center hover:border-gray-300 transition-colors cursor-pointer"
    >
      {/* Shift */}
      <span className={`text-2xl ${hasShift ? 'text-gray-700' : 'text-gray-300'}`}>⇧</span>

      {/* Option */}
      <span className={`text-2xl ${hasOption ? 'text-gray-700' : 'text-gray-300'}`}>⌥</span>

      {/* Command */}
      <span className={`text-2xl ${hasCommand ? 'text-gray-700' : 'text-gray-300'}`}>⌘</span>

      {/* Control */}
      <span className={`text-2xl ${hasControl ? 'text-gray-700' : 'text-gray-300'}`}>⌃</span>

      {/* 字母键 */}
      {letterKey && (
        <span className="text-2xl font-medium text-blue-500 ml-2">
          {letterKey}
        </span>
      )}
    </button>
  );
}
