import { useState } from 'react';
import type { InterfaceTheme } from '../../../application/settings/theme';

const themes = [
  { id: 'native', name: '澄明', english: 'Native', description: '清透蓝调 · 经典侧栏' },
  { id: 'studio', name: '夜航', english: 'Studio', description: '琥珀强调 · 紧凑工作台' },
  { id: 'paper', name: '松间', english: 'Paper', description: '暖纸松绿 · 舒展阅读' },
] as const;

export function ThemePicker({
  value,
  onChange,
}: {
  value: InterfaceTheme;
  onChange: (theme: InterfaceTheme) => Promise<unknown>;
}) {
  const [pending, setPending] = useState<InterfaceTheme | null>(null);
  const [error, setError] = useState(false);

  const select = async (theme: InterfaceTheme) => {
    if (pending || value === theme) return;
    setPending(theme);
    setError(false);
    try {
      await onChange(theme);
    } catch {
      setError(true);
    } finally {
      setPending(null);
    }
  };

  return (
    <div className="theme-picker" aria-busy={pending !== null}>
      <div className="theme-options" role="group" aria-label="界面主题">
        {themes.map((theme) => (
          <button
            key={theme.id}
            type="button"
            className="theme-option"
            data-theme-option={theme.id}
            aria-label={`${theme.name} · ${theme.english}`}
            aria-pressed={value === theme.id}
            disabled={pending !== null}
            onClick={() => void select(theme.id)}
          >
            <span className="theme-mini" aria-hidden="true">
              <span className="theme-mini-chrome"><i /><i /><i /></span>
              <span className="theme-mini-nav"><i /><i /><i /><i /></span>
              <span className="theme-mini-content"><b /><i /><i /><i /></span>
            </span>
            <span className="theme-option-name">
              <strong>{theme.name}</strong><small>{theme.english}</small>
              <span className="theme-option-check" aria-hidden="true">{value === theme.id ? '✓' : ''}</span>
            </span>
            <span className="theme-option-description">{theme.description}</span>
          </button>
        ))}
      </div>
      {error && <p role="alert" className="mt-3 text-xs text-danger">主题保存失败，请重试。</p>}
    </div>
  );
}
