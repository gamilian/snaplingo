import { useEffect, useRef, useState } from 'react';

interface ScreenshotCaptureProps {
  screenImage: string;
  onCapture: (region: { x: number; y: number; width: number; height: number }) => void;
  onCancel: () => void;
}

export function ScreenshotCapture({ screenImage, onCapture, onCancel }: ScreenshotCaptureProps) {
  const [isSelecting, setIsSelecting] = useState(false);
  const [startPos, setStartPos] = useState<{ x: number; y: number } | null>(null);
  const [currentPos, setCurrentPos] = useState<{ x: number; y: number } | null>(null);
  const startPosRef = useRef<{ x: number; y: number } | null>(null);

  // 处理鼠标按下
  const handlePointerDown = (e: React.PointerEvent<HTMLDivElement>) => {
    e.currentTarget.setPointerCapture(e.pointerId);
    const position = { x: e.clientX, y: e.clientY };

    setIsSelecting(true);
    startPosRef.current = position;
    setStartPos(position);
    setCurrentPos(position);
  };

  // 处理鼠标移动
  const handlePointerMove = (e: React.PointerEvent<HTMLDivElement>) => {
    if (isSelecting && startPosRef.current) {
      setCurrentPos({ x: e.clientX, y: e.clientY });
    }
  };

  // 处理鼠标释放
  const handlePointerUp = (e: React.PointerEvent<HTMLDivElement>) => {
    const start = startPosRef.current;

    if (isSelecting && start) {
      const x = Math.min(start.x, e.clientX);
      const y = Math.min(start.y, e.clientY);
      const width = Math.abs(e.clientX - start.x);
      const height = Math.abs(e.clientY - start.y);

      if (width > 10 && height > 10) {
        onCapture({ x, y, width, height });
      }
    }

    startPosRef.current = null;
    setIsSelecting(false);
  };

  const handlePointerCancel = () => {
    startPosRef.current = null;
    setIsSelecting(false);
  };

  // 处理 ESC 键取消
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        onCancel();
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [onCancel]);

  // 计算选区位置和尺寸
  const getSelectionRect = () => {
    if (!startPos || !currentPos) return null;

    const x = Math.min(startPos.x, currentPos.x);
    const y = Math.min(startPos.y, currentPos.y);
    const width = Math.abs(currentPos.x - startPos.x);
    const height = Math.abs(currentPos.y - startPos.y);

    return { x, y, width, height };
  };

  const selectionRect = getSelectionRect();

  return (
    <div
      className="fixed inset-0 z-[9999] cursor-crosshair"
      onPointerDown={handlePointerDown}
      onPointerMove={handlePointerMove}
      onPointerUp={handlePointerUp}
      onPointerCancel={handlePointerCancel}
      style={{ backgroundColor: 'transparent' }}
    >
      {/* 背景截图 */}
      <img
        src={screenImage}
        alt="Screen"
        className="absolute inset-0 w-full h-full object-cover"
      />

      {/* 选区高亮 */}
      {selectionRect && (
        <>
          {/* 选中的区域（蓝色边框） */}
          <div
            className="absolute"
            style={{
              left: selectionRect.x,
              top: selectionRect.y,
              width: selectionRect.width,
              height: selectionRect.height,
              border: '3px solid #5b7fff',
              boxShadow: '0 8px 24px rgba(0, 0, 0, 0.12)',
              pointerEvents: 'none',
            }}
          >
            {/* 尺寸提示 */}
            <div className="absolute left-0 top-full mt-2 bg-blue-500 text-white text-xs px-2 py-1 rounded whitespace-nowrap">
              {selectionRect.width} × {selectionRect.height}
            </div>
          </div>
        </>
      )}
    </div>
  );
}
