import { useState } from 'react';

interface TranslationCardProps {
  providerId: string;
  text: string;
  detectedLanguage?: string;
}

export default function TranslationCard({
  providerId,
  text,
  detectedLanguage,
}: TranslationCardProps) {
  const [isExpanded, setIsExpanded] = useState(true);

  return (
    <div className="border border-gray-200 rounded-lg overflow-hidden">
      <button
        onClick={() => setIsExpanded(!isExpanded)}
        className="w-full px-4 py-3 bg-gray-50 hover:bg-gray-100 flex items-center justify-between text-left transition-colors"
      >
        <div className="flex items-center gap-2">
          <span className="font-medium text-gray-900">{providerId}</span>
          {detectedLanguage && (
            <span className="text-xs text-gray-500 bg-gray-200 px-2 py-1 rounded">
              Detected: {detectedLanguage}
            </span>
          )}
        </div>
        <svg
          className={`w-5 h-5 text-gray-500 transition-transform ${
            isExpanded ? 'rotate-180' : ''
          }`}
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24"
        >
          <path
            strokeLinecap="round"
            strokeLinejoin="round"
            strokeWidth={2}
            d="M19 9l-7 7-7-7"
          />
        </svg>
      </button>

      {isExpanded && (
        <div className="p-4 bg-white">
          <p className="text-gray-800 whitespace-pre-wrap">{text}</p>
        </div>
      )}
    </div>
  );
}
