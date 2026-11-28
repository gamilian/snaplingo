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
  const hasWin = value.includes('⊞');
  const isWindows = typeof navigator !== 'undefined' && navigator.platform.startsWith('Win');
  const modifiers = [
    { label: isWindows ? 'Shift' : '⇧', active: hasShift },
    { label: isWindows ? 'Alt' : '⌥', active: hasOption },
    { label: isWindows ? 'Ctrl' : '⌘', active: isWindows ? hasCommand || hasControl : hasCommand },
    { label: isWindows ? 'Win' : '⌃', active: isWindows ? hasWin : hasControl },
  ];

  // 提取字母键（最后一个非修饰键字符）
  const letterKey = value.replace(/[⇧⌥⌘⌃⊞]/g, '').trim();

  const isUnset = value === "未设置";
  const activeKeys = [...modifiers.filter(({ active }) => active).map(({ label }) => label), letterKey];

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

  return (
    <button
      type="button"
      onClick={handleClick}
      className={containerClass}
      aria-label={isRecording ? '请按下快捷键，Esc 取消' : isUnset ? '设置快捷键' : `修改快捷键：${activeKeys.join(' + ')}`}
      title={isRecording ? '按 Esc 取消录制' : '点击修改快捷键'}
    >
      {modifiers.map(({ label, active }) => (
        <kbd key={label} aria-hidden="true" className={`font-[inherit] text-base ${!isUnset && !isRecording && active ? 'text-ink' : 'text-faint'}`}>
          {label}
        </kbd>
      ))}
      {isRecording || isUnset ? (
        <span className={`ml-1 whitespace-nowrap text-hint text-faint${isRecording ? ' animate-pulse' : ''}`}>
          {isRecording ? '按下快捷键...' : '未设置'}
        </span>
      ) : letterKey && (
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
