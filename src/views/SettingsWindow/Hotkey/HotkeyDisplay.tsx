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
    ? 'text-ink'
    : 'text-faint';
  const shiftLabel = isWindows ? 'Shift' : '⇧';
  const optionLabel = isWindows ? 'Alt' : '⌥';
  const primaryLabel = isWindows ? 'Ctrl' : '⌘';

  // 提取字母键（最后一个非修饰键字符）
  const letterKey = value.replace(/[⇧⌥⌘⌃]/g, '').trim();

  const isUnset = value === "未设置";

  // 统一的容器样式
  const containerClass = `
    flex h-9 min-w-[156px] items-center justify-center space-x-1 rounded-lg border bg-surface px-3
    transition-all duration-200 cursor-pointer
    ${isRecording
      ? 'border-accent ring-2 ring-accent-line shadow-sm animate-pulse'
      : isUnset
        ? 'border-2 border-dashed border-line-strong hover:border-faint'
        : 'border border-line hover:border-line-strong hover:shadow-sm'
    }
  `;

  if (isUnset || isRecording) {
    return (
      <button onClick={handleClick} className={containerClass}>
        {isRecording ? (
          <>
            {/* 录制中显示所有修饰键（灰色） */}
            <span className="text-base text-faint">{shiftLabel}</span>
            <span className="text-base text-faint">{optionLabel}</span>
            <span className="text-base text-faint">{primaryLabel}</span>
            {!isWindows && <span className="text-base text-faint">⌃</span>}
            <span className="ml-1 text-hint text-faint animate-pulse">按下快捷键...</span>
          </>
        ) : (
          <>
            {/* 未设置显示所有修饰键（灰色） */}
            <span className="text-base text-faint">{shiftLabel}</span>
            <span className="text-base text-faint">{optionLabel}</span>
            <span className="text-base text-faint">{primaryLabel}</span>
            {!isWindows && <span className="text-base text-faint">⌃</span>}
            <span className="ml-1 text-hint text-faint">未设置</span>
          </>
        )}
      </button>
    );
  }

  return (
    <button onClick={handleClick} className={containerClass}>
      {/* Shift */}
      <span className={`text-base ${hasShift ? 'text-ink' : 'text-faint'}`}>{shiftLabel}</span>

      {/* Option */}
      <span className={`text-base ${hasOption ? 'text-ink' : 'text-faint'}`}>{optionLabel}</span>

      {/* Primary modifier */}
      <span className={`text-base ${primaryModifierClass}`}>
        {primaryLabel}
      </span>

      {/* Control */}
      {!isWindows && (
        <span className={`text-base ${hasControl ? 'text-ink' : 'text-faint'}`}>⌃</span>
      )}

      {/* 字母键 */}
      {letterKey && (
        <span
          className="ml-1 text-base font-semibold text-accent"
          style={{
            fontFamily: '"Segoe UI Variable Text", "Segoe UI", system-ui, "Microsoft YaHei", sans-serif, "Apple Color Emoji", "Segoe UI Emoji"',
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
