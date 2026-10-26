import type { ReactNode } from 'react';

export function SettingsGroup({
  title,
  children,
}: {
  title?: string;
  children: ReactNode;
}) {
  return (
    <section className="settings-group border-t border-line first:border-t-0">
      {title && (
        <div className="settings-group-label border-b border-line bg-sunken px-[22px] py-2 text-cap font-semibold uppercase tracking-[0.075em] text-faint">
          {title}
        </div>
      )}
      <div className="divide-y divide-line px-[22px]">{children}</div>
    </section>
  );
}

export function SettingRow({
  label,
  description,
  children,
}: {
  label: string;
  description?: string;
  children: ReactNode;
}) {
  return (
    <div className="settings-control-row grid min-h-[54px] grid-cols-[minmax(0,1fr)_minmax(200px,auto)] items-center gap-x-6 gap-y-2 py-2.5">
      <div>
        <div className="text-label font-semibold text-ink">{label}</div>
        {description && (
          <div className="mt-0.5 text-hint leading-relaxed text-muted">
            {description}
          </div>
        )}
      </div>
      <div className="justify-self-end">{children}</div>
    </div>
  );
}

export function SettingsToggle({
  checked,
  onChange,
  label,
}: {
  checked: boolean;
  onChange: (checked: boolean) => void;
  label: string;
}) {
  return (
    <button
      type="button"
      role="switch"
      aria-label={label}
      aria-checked={checked}
      onClick={() => onChange(!checked)}
      className={`relative h-5 w-9 rounded-full transition-colors ${
        checked ? 'bg-accent' : 'bg-line-strong'
      }`}
    >
      <span
        className={`absolute left-0.5 top-0.5 h-4 w-4 rounded-full bg-surface shadow transition-transform ${
          checked ? 'translate-x-4' : ''
        }`}
      />
    </button>
  );
}
