interface CustomRangeProps {
  value: number;
  onChange: (value: number) => void;
  min: number;
  max: number;
  step?: number;
  className?: string;
}

export function CustomRange({
  value,
  onChange,
  min,
  max,
  step = 1,
  className = ''
}: CustomRangeProps) {
  const percentage = ((value - min) / (max - min)) * 100;

  return (
    <div className={`relative w-full ${className}`}>
      <input
        type="range"
        min={min}
        max={max}
        step={step}
        value={value}
        onChange={(e) => onChange(Number(e.target.value))}
        className="smooth-range"
        style={{
          ['--range-progress' as any]: `${percentage}%`
        }}
      />

      <style>{`
        /* 轨道容器 */
        .smooth-range {
          -webkit-appearance: none;
          appearance: none;
          width: 100%;
          height: 6px;
          background: transparent;
          outline: none;
          cursor: pointer;
          position: relative;
        }

        /* 轨道背景 - 无过渡，即时响应 */
        .smooth-range::before {
          content: '';
          position: absolute;
          left: 0;
          top: 50%;
          transform: translateY(-50%);
          width: 100%;
          height: 6px;
          background: linear-gradient(
            to right,
            #5b7fff 0%,
            #5b7fff var(--range-progress),
            #e5e7eb var(--range-progress),
            #e5e7eb 100%
          );
          border-radius: 3px;
          pointer-events: none;
          will-change: background;
        }

        /* Chrome/Safari 滑块 - 只动画缩放，不动画位置 */
        .smooth-range::-webkit-slider-thumb {
          -webkit-appearance: none;
          appearance: none;
          width: 18px;
          height: 18px;
          border-radius: 50%;
          background: white;
          border: 2px solid #5b7fff;
          box-shadow: 0 2px 6px rgba(0, 0, 0, 0.15);
          cursor: grab;
          position: relative;
          z-index: 1;

          /* 性能优化 */
          will-change: transform;
          transform: scale(1) translateZ(0);

          /* 只给 transform 和 box-shadow 添加过渡，位置变化无过渡 */
          transition: transform 150ms cubic-bezier(0.4, 0, 0.2, 1),
                      box-shadow 150ms cubic-bezier(0.4, 0, 0.2, 1);

          /* 强制GPU加速 */
          backface-visibility: hidden;
          -webkit-font-smoothing: subpixel-antialiased;
        }

        .smooth-range:hover::-webkit-slider-thumb {
          transform: scale(1.2) translateZ(0);
          box-shadow: 0 4px 10px rgba(91, 127, 255, 0.35);
        }

        .smooth-range:active::-webkit-slider-thumb {
          transform: scale(1.3) translateZ(0);
          cursor: grabbing;
          box-shadow: 0 6px 16px rgba(91, 127, 255, 0.45);
        }

        /* Firefox 滑块 */
        .smooth-range::-moz-range-thumb {
          width: 18px;
          height: 18px;
          border-radius: 50%;
          background: white;
          border: 2px solid #5b7fff;
          box-shadow: 0 2px 6px rgba(0, 0, 0, 0.15);
          cursor: grab;

          will-change: transform;
          transform: scale(1) translateZ(0);
          transition: transform 150ms cubic-bezier(0.4, 0, 0.2, 1),
                      box-shadow 150ms cubic-bezier(0.4, 0, 0.2, 1);
        }

        .smooth-range:hover::-moz-range-thumb {
          transform: scale(1.2) translateZ(0);
          box-shadow: 0 4px 10px rgba(91, 127, 255, 0.35);
        }

        .smooth-range:active::-moz-range-thumb {
          transform: scale(1.3) translateZ(0);
          cursor: grabbing;
          box-shadow: 0 6px 16px rgba(91, 127, 255, 0.45);
        }

        /* Firefox 轨道 */
        .smooth-range::-moz-range-track {
          background: transparent;
          border: none;
        }

        /* 移除Firefox的默认进度 */
        .smooth-range::-moz-range-progress {
          display: none;
        }
      `}</style>
    </div>
  );
}
