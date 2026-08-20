interface HotkeyDisplayProps {
  value: string; // e.g., "⌥S", "⇧⌥S", "未设置"
  onClick?: () => void;
  isRecording?: boolean;
}

export function HotkeyDisplay({ value, onClick, isRecording = false }: HotkeyDisplayProps) {
  const handleClick = () => {
    onClick?.();
  };

  // 解析快捷键
  const hasShift = value.includes('⇧');
  const hasOption = value.includes('⌥');
  const hasCommand = value.includes('⌘');
  const hasControl = value.includes('⌃');
  const isWindows = typeof navigator !== 'undefined' && navigator.platform.startsWith('Win');
  const hasWindowsControl = hasCommand || hasControl;
  const primaryModifierClass = (isWindows ? hasWindowsControl : hasCommand)
    ? 'text-gray-700'
    : 'text-gray-300';

  // 提取字母键（最后一个非修饰键字符）
  const letterKey = value.replace(/[⇧⌥⌘⌃]/g, '').trim();

  const isUnset = value === "未设置";

  // 统一的容器样式
  const containerClass = `
    flex h-9 min-w-[156px] items-center justify-center space-x-1 rounded-lg border bg-white px-3
    transition-all duration-200 cursor-pointer
    ${isRecording
      ? 'border-primary-500 ring-2 ring-primary-100 shadow-sm animate-pulse'
      : isUnset
        ? 'border-2 border-dashed border-gray-300 hover:border-gray-400'
        : 'border border-gray-200 hover:border-gray-300 hover:shadow-sm'
    }
  `;

  if (isUnset) {
    return (
      <button onClick={handleClick} className={containerClass}>
        {isRecording ? (
          <>
            {/* 录制中显示所有修饰键（灰色） */}
            <span className="text-base text-gray-300">⇧</span>
            <span className="text-base text-gray-300">⌥</span>
            <span className="text-base text-gray-300">{isWindows ? 'Ctrl' : '⌘'}</span>
            {!isWindows && <span className="text-base text-gray-300">⌃</span>}
            <span className="ml-1 text-[11px] text-gray-400 animate-pulse">按下快捷键...</span>
          </>
        ) : (
          <>
            {/* 未设置显示所有修饰键（灰色） */}
            <span className="text-base text-gray-300">⇧</span>
            <span className="text-base text-gray-300">⌥</span>
            <span className="text-base text-gray-300">{isWindows ? 'Ctrl' : '⌘'}</span>
            {!isWindows && <span className="text-base text-gray-300">⌃</span>}
            <span className="ml-1 text-[11px] text-gray-400">未设置</span>
          </>
        )}
      </button>
    );
  }

  return (
    <button onClick={handleClick} className={containerClass}>
      {/* Shift */}
      <span className={`text-base ${hasShift ? 'text-gray-700' : 'text-gray-300'}`}>⇧</span>

      {/* Option */}
      <span className={`text-base ${hasOption ? 'text-gray-700' : 'text-gray-300'}`}>⌥</span>

      {/* Primary modifier */}
      <span className={`text-base ${primaryModifierClass}`}>
        {isWindows ? 'Ctrl' : '⌘'}
      </span>

      {/* Control */}
      {!isWindows && (
        <span className={`text-base ${hasControl ? 'text-gray-700' : 'text-gray-300'}`}>⌃</span>
      )}

      {/* 字母键 */}
      {letterKey && (
        <span
          className="ml-1 text-base font-semibold text-primary-500"
          style={{
            fontFamily: '-apple-system, BlinkMacSystemFont, "Segoe UI", "Roboto", "Helvetica Neue", Arial, "Noto Sans", sans-serif, "Apple Color Emoji", "Segoe UI Emoji"',
            fontStyle: 'normal',
            letterSpacing: '0.02em'
          }}
        >
          {letterKey}
        </span>
      )}
    </button>
  );
}
