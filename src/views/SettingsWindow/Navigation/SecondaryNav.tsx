interface SecondaryNavProps {
  items: Array<{ key: string; label: string }>;
  activeItem: string;
  onItemChange: (key: string) => void;
  orientation?: 'vertical' | 'horizontal';
}

export function SecondaryNav({
  items,
  activeItem,
  onItemChange,
  orientation = 'vertical',
}: SecondaryNavProps) {
  if (orientation === 'horizontal') {
    return (
      <div className="inline-flex max-w-[60%] shrink overflow-x-auto rounded-md bg-raised p-[3px]">
        <nav className="flex gap-0.5">
          {items.map((item) => {
            const active = activeItem === item.key;
            return (
              <button
                key={item.key}
                type="button"
                aria-current={active ? 'page' : undefined}
                onClick={() => onItemChange(item.key)}
                className={`
                  h-[25px] shrink-0 whitespace-nowrap rounded-sm px-2.5 text-label font-medium transition-colors
                  ${
                    active
                      ? 'bg-surface text-ink shadow-xs'
                      : 'text-muted hover:text-ink'
                  }
                `}
              >
                {item.label}
              </button>
            );
          })}
        </nav>
      </div>
    );
  }

  return (
    <div className="flex w-[200px] flex-col border-r border-line bg-sunken">
      <nav className="flex-1 space-y-0.5 p-4">
        {items.map((item) => {
          const active = activeItem === item.key;
          return (
            <button
              key={item.key}
              type="button"
              aria-current={active ? 'page' : undefined}
              onClick={() => onItemChange(item.key)}
              className={`
                relative w-full rounded-md px-2.5 py-1.5 text-left text-label transition-colors
                ${
                  active
                    ? 'bg-accent-soft font-semibold text-accent-text'
                    : 'text-muted hover:bg-raised/70 hover:text-ink'
                }
              `}
            >
              {active && (
                <span
                  aria-hidden="true"
                  className="absolute -left-4 top-1.5 bottom-1.5 w-0.5 rounded-r-sm bg-accent"
                />
              )}
              {item.label}
            </button>
          );
        })}
      </nav>
    </div>
  );
}
