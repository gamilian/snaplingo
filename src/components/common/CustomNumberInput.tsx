import { useState } from 'react';

interface CustomNumberInputProps {
  value: number;
  onChange: (value: number) => void;
  min?: number;
  max?: number;
  step?: number;
  placeholder?: string;
  className?: string;
  suffix?: string;
}

export function CustomNumberInput({
  value,
  onChange,
  min,
  max,
  step = 1,
  placeholder,
  className = '',
  suffix,
}: CustomNumberInputProps) {
  const [isFocused, setIsFocused] = useState(false);

  const handleIncrement = () => {
    const newValue = value + step;
    if (max === undefined || newValue <= max) {
      onChange(newValue);
    }
  };

  const handleDecrement = () => {
    const newValue = value - step;
    if (min === undefined || newValue >= min) {
      onChange(newValue);
    }
  };

  const handleInputChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const newValue = parseFloat(e.target.value);
    if (!isNaN(newValue)) {
      if ((min === undefined || newValue >= min) && (max === undefined || newValue <= max)) {
        onChange(newValue);
      }
    } else if (e.target.value === '') {
      onChange(min || 0);
    }
  };

  const canIncrement = max === undefined || value < max;
  const canDecrement = min === undefined || value > min;

  return (
    <div className={`relative ${className}`}>
      <input
        type="number"
        value={value}
        onChange={handleInputChange}
        onFocus={() => setIsFocused(true)}
        onBlur={() => setIsFocused(false)}
        placeholder={placeholder}
        min={min}
        max={max}
        step={step}
        className={`
          w-full px-4 py-2.5 pr-12 bg-surface border rounded-lg text-sm
          transition-all duration-200
          ${isFocused
            ? 'border-accent ring-2 ring-accent-line shadow-sm'
            : 'border-line hover:border-line-strong hover:shadow-sm'
          }
          focus:outline-none
          [appearance:textfield]
          [&::-webkit-outer-spin-button]:appearance-none
          [&::-webkit-inner-spin-button]:appearance-none
        `}
      />

      {/* 后缀文字 */}
      {suffix && (
        <div className="absolute right-14 top-1/2 -translate-y-1/2 text-sm text-muted pointer-events-none">
          {suffix}
        </div>
      )}

      {/* 自定义增减按钮 */}
      <div className="absolute right-2 top-1/2 -translate-y-1/2 flex flex-col">
        <button
          type="button"
          onClick={handleIncrement}
          disabled={!canIncrement}
          className={`
            w-6 h-5 flex items-center justify-center rounded-sm
            transition-all duration-150
            ${canIncrement
              ? 'hover:bg-raised text-muted hover:text-ink'
              : 'text-faint cursor-not-allowed'
            }
          `}
          tabIndex={-1}
        >
          <svg className="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24" strokeWidth="2">
            <path strokeLinecap="round" strokeLinejoin="round" d="M5 15l7-7 7 7" />
          </svg>
        </button>

        <button
          type="button"
          onClick={handleDecrement}
          disabled={!canDecrement}
          className={`
            w-6 h-5 flex items-center justify-center rounded-sm
            transition-all duration-150
            ${canDecrement
              ? 'hover:bg-raised text-muted hover:text-ink'
              : 'text-faint cursor-not-allowed'
            }
          `}
          tabIndex={-1}
        >
          <svg className="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24" strokeWidth="2">
            <path strokeLinecap="round" strokeLinejoin="round" d="M19 9l-7 7-7-7" />
          </svg>
        </button>
      </div>
    </div>
  );
}
