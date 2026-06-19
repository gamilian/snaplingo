import { useEffect } from 'react';

interface HotkeyDisplayProps {
  value: string; // e.g., "⌥S", "⇧⌥S", "未设置"
  onClick?: () => void;
  isRecording?: boolean;
}

export function HotkeyDisplay({ value, onClick, isRecording = false }: HotkeyDisplayProps) {
  // 录制动画效果
  useEffect(() => {
    if (isRecording) {
      const interval = setInterval(() => {
        // Pulse animation handled by CSS
      }, 800);
      return () => clearInterval(interval);
    }
  }, [isRecording]);

  const handleClick = () => {
    console.log('[HotkeyDisplay] Clicked!', {
      value,
      isRecording,
      isUnset,
      hasOnClick: !!onClick
    });
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

  // 统一的容器样式
  const containerClass = `
    flex items-center justify-center space-x-1 px-4 py-2 bg-white border rounded-lg
    min-w-[280px] transition-all duration-200 cursor-pointer
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
            <span className="text-2xl text-gray-300">⇧</span>
            <span className="text-2xl text-gray-300">⌥</span>
            <span className="text-2xl text-gray-300">⌘</span>
            <span className="text-2xl text-gray-300">⌃</span>
            <span className="text-sm text-gray-400 ml-2 animate-pulse">按下快捷键...</span>
          </>
        ) : (
          <>
            {/* 未设置显示所有修饰键（灰色） */}
            <span className="text-2xl text-gray-300">⇧</span>
            <span className="text-2xl text-gray-300">⌥</span>
            <span className="text-2xl text-gray-300">⌘</span>
            <span className="text-2xl text-gray-300">⌃</span>
            <span className="text-sm text-gray-400 ml-2">未设置</span>
          </>
        )}
      </button>
    );
  }

  return (
    <button onClick={handleClick} className={containerClass}>
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
        <span
          className="text-2xl font-semibold text-primary-500 ml-2"
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
