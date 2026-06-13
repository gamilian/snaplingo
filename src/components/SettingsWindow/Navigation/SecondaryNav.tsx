interface SecondaryNavProps {
  items: Array<{ key: string; label: string }>;
  activeItem: string;
  onItemChange: (key: string) => void;
}

export function SecondaryNav({ items, activeItem, onItemChange }: SecondaryNavProps) {
  return (
    <div className="w-56 bg-gray-50 border-r border-gray-200 flex flex-col">
      <nav className="flex-1 p-6 space-y-2">
        {items.map((item) => (
          <button
            key={item.key}
            onClick={() => onItemChange(item.key)}
            className={`w-full text-left px-4 py-2.5 rounded-lg text-sm font-medium transition-colors ${
              activeItem === item.key
                ? 'bg-blue-500 text-white shadow-sm'
                : 'text-gray-700 hover:bg-gray-100'
            }`}
          >
            {item.label}
          </button>
        ))}
      </nav>
    </div>
  );
}
