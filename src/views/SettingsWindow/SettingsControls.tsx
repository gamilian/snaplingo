import type { ReactNode } from 'react';

export function SettingsGroup({
  title,
  children,
}: {
  title: string;
  children: ReactNode;
}) {
  return (
    <section className="border-t border-gray-200 first:border-t-0">
      <div className="border-b border-gray-100 bg-gray-50 px-[22px] py-2.5 text-[10px] font-bold tracking-[0.04em] text-gray-500">
        {title}
      </div>
      <div className="divide-y divide-gray-100 px-[22px]">{children}</div>
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
    <div className="grid min-h-[70px] grid-cols-[minmax(0,1fr)_minmax(230px,auto)] items-center gap-9">
      <div>
        <div className="text-[13px] font-semibold text-gray-800">{label}</div>
        {description && (
          <div className="mt-1 text-[11px] leading-relaxed text-gray-500">
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
      className={`relative h-[22px] w-10 rounded-full transition-colors ${
        checked ? 'bg-primary-600' : 'bg-gray-300'
      }`}
    >
      <span
        className={`absolute left-0.5 top-0.5 h-[18px] w-[18px] rounded-full bg-white shadow transition-transform ${
          checked ? 'translate-x-[18px]' : ''
        }`}
      />
    </button>
  );
}
