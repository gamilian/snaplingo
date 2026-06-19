import { useState, useEffect } from 'react';

interface ScreenshotEditorProps {
  image: string;
  region: { x: number; y: number; width: number; height: number };
  onComplete: (action: 'save' | 'copy' | 'pin') => void;
  onCancel: () => void;
}

type Tool = 'select' | 'rectangle' | 'ellipse' | 'arrow' | 'pen' | 'text' | 'mosaic' | 'blur';

export function ScreenshotEditor({ image, region, onComplete, onCancel }: ScreenshotEditorProps) {
  const [selectedTool, setSelectedTool] = useState<Tool>('select');

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        onCancel();
      } else if (e.key === 'Enter') {
        onComplete('copy');
      } else if ((e.metaKey || e.ctrlKey) && e.key === 'c') {
        e.preventDefault();
        onComplete('copy');
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [onCancel, onComplete]);

  const tools = [
    { id: 'select' as Tool, icon: '↖️', label: '选择' },
    { id: 'rectangle' as Tool, icon: '▭', label: '矩形' },
    { id: 'ellipse' as Tool, icon: '○', label: '椭圆' },
    { id: 'arrow' as Tool, icon: '→', label: '箭头' },
    { id: 'pen' as Tool, icon: '✏️', label: '画笔' },
    { id: 'text' as Tool, icon: 'T', label: '文字' },
    { id: 'mosaic' as Tool, icon: '▦', label: '马赛克' },
    { id: 'blur' as Tool, icon: '◐', label: '模糊' },
  ];

  return (
    <div className="fixed inset-0 z-[9999] flex flex-col items-center justify-center gap-3" style={{ background: 'rgba(0, 0, 0, 0.95)' }}>
      {/* 截图预览区域 */}
      <div
        className="relative flex items-center justify-center rounded-lg"
        style={{
          width: region.width,
          height: region.height,
          maxWidth: '90vw',
          maxHeight: '70vh',
          border: '3px solid #5b7fff',
          boxShadow: '0 12px 40px rgba(0, 0, 0, 0.16)',
          background: '#f4f6f8',
        }}
      >
        <img src={image} alt="Screenshot" className="w-full h-full object-contain" />
        <div className="absolute inset-0 flex items-center justify-center text-gray-400 text-sm pointer-events-none">
          {region.width} × {region.height}
        </div>
      </div>

      {/* 工具栏 + 操作栏 */}
      <div className="bg-white rounded-xl p-2 flex gap-3 items-center shadow-lg border border-gray-200">
        {/* 左侧工具图标 */}
        <div className="flex gap-1 items-center">
          {tools.map((tool) => (
            <button
              key={tool.id}
              onClick={() => setSelectedTool(tool.id)}
              title={tool.label}
              className={`w-9 h-9 flex items-center justify-center rounded-lg text-lg transition-all ${
                selectedTool === tool.id
                  ? 'bg-blue-50 border-2 border-blue-500 text-blue-600'
                  : 'border-2 border-transparent text-gray-600 hover:bg-gray-100'
              }`}
            >
              {tool.icon}
            </button>
          ))}

          {/* 颜色选择器 */}
          <div
            className="w-9 h-9 rounded-lg border-2 border-white cursor-pointer ml-1"
            style={{
              background: '#5b7fff',
              boxShadow: '0 0 0 1px #dde2e8',
            }}
          />
        </div>

        {/* 分隔线 */}
        <div className="w-px h-9 bg-gray-200" />

        {/* 右侧操作按钮 */}
        <div className="flex gap-2 items-center">
          <button
            onClick={onCancel}
            className="px-4 py-2 bg-white border border-gray-300 rounded-lg text-gray-700 text-xs font-medium hover:bg-gray-50 transition-all"
          >
            取消 (Esc)
          </button>
          <button
            onClick={() => console.log('OCR')}
            className="px-4 py-2 bg-white border border-gray-300 rounded-lg text-gray-700 text-xs font-medium hover:bg-gray-50 transition-all"
          >
            OCR
          </button>
          <button
            onClick={() => onComplete('copy')}
            className="px-4 py-2 bg-white border border-gray-300 rounded-lg text-gray-700 text-xs font-medium hover:bg-gray-50 transition-all"
          >
            复制 (⌘C)
          </button>
          <button
            onClick={() => onComplete('save')}
            className="px-4 py-2 bg-white border border-gray-300 rounded-lg text-gray-700 text-xs font-medium hover:bg-gray-50 transition-all"
          >
            保存
          </button>
          <button
            onClick={() => onComplete('copy')}
            className="px-5 py-2 rounded-lg text-white text-xs font-semibold shadow-sm transition-all"
            style={{
              background: 'linear-gradient(135deg, #5b7fff 0%, #4a6fe8 100%)',
            }}
          >
            完成 (Enter)
          </button>
        </div>
      </div>
    </div>
  );
}
