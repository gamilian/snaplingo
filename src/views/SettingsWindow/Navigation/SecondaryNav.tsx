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
      <div className="inline-flex rounded-[22px] border border-gray-200 bg-white/80 p-1.5 shadow-sm backdrop-blur">
        <nav className="flex gap-1.5">
          {items.map((item) => (
            <button
              key={item.key}
              onClick={() => onItemChange(item.key)}
              className={`
                rounded-2xl px-4 py-2 text-sm font-medium transition-all duration-150
                ${
                  activeItem === item.key
                    ? 'bg-primary-600 text-white shadow-sm'
                    : 'text-gray-500 hover:bg-primary-50 hover:text-primary-600'
                }
              `}
            >
              {item.label}
            </button>
          ))}
        </nav>
      </div>
    );
  }

  return (
    <div className="w-[200px] bg-gray-50 border-r border-gray-200 flex flex-col">
      <nav className="flex-1 p-6 space-y-1">
        {items.map((item) => (
          <button
            key={item.key}
            onClick={() => onItemChange(item.key)}
            className={`
              w-full text-left px-3 py-2 rounded-md text-sm font-medium
              transition-all duration-150
              border-l-2
              ${
                activeItem === item.key
                  ? 'bg-white text-gray-900 font-semibold border-primary-500 shadow-xs'
                  : 'text-gray-600 hover:bg-white hover:text-gray-900 border-transparent hover:border-gray-300'
              }
            `}
          >
            {item.label}
          </button>
        ))}
      </nav>
    </div>
  );
}
