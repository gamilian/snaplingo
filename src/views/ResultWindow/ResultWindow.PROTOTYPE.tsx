/**
 * PROTOTYPE - THROWAWAY CODE
 *
 * This file contains 4 radially different UI variations for the Translation Window.
 * Switch between them using the floating bar at the bottom.
 *
 * Question: What should the translation window look/feel like?
 *
 * Variations:
 * 1. Bob-style: Compact toolbar, multiple expandable cards (from UI doc)
 * 2. Split Panel: Side-by-side source/results view
 * 3. Minimal: Ultra-clean, single result focus, minimal chrome
 * 4. Card Stack: Swipeable Instagram-story style cards
 *
 * DELETE OR ABSORB WHEN DONE
 */

import { useState, useEffect } from 'react';
import { useAppStore } from '../../stores/appStore';

const LANGUAGES = [
  { code: 'auto', name: '自动检测' },
  { code: 'en', name: 'English' },
  { code: 'zh-CN', name: '中文简体' },
  { code: 'zh-TW', name: '中文繁體' },
  { code: 'ja', name: '日本語' },
  { code: 'ko', name: '한국어' },
];

// Mock translation results for prototype
const MOCK_TRANSLATIONS = [
  { provider_id: 'deeplx', name: 'DeepLX', text: 'Accurate identification', icon: 'D' },
  { provider_id: 'google', name: 'Google 翻译', text: 'Accurate identification', icon: 'G' },
  { provider_id: 'openai', name: 'GPT-4', text: 'Precise recognition', icon: 'O' },
];

type Variant = 'bob' | 'split' | 'minimal' | 'stack';

export default function ResultWindowPrototype() {
  const [variant, setVariant] = useState<Variant>('bob');
  const {
    sourceText,
    sourceLang,
    targetLang,
    resultWindowVisible,
    setSourceText,
    setSourceLang,
    setTargetLang,
    hideResultWindow,
  } = useAppStore();

  useEffect(() => {
    const handleEscape = (e: KeyboardEvent) => {
      if (e.key === 'Escape' && resultWindowVisible) hideResultWindow();
    };
    window.addEventListener('keydown', handleEscape);
    return () => window.removeEventListener('keydown', handleEscape);
  }, [resultWindowVisible, hideResultWindow]);

  if (!resultWindowVisible) return null;

  const handleOverlayClick = (e: React.MouseEvent) => {
    if (e.target === e.currentTarget) hideResultWindow();
  };

  return (
    <div className="fixed inset-0 bg-black bg-opacity-30 flex items-center justify-center z-50" onClick={handleOverlayClick}>
      {variant === 'bob' && <BobStyleVariant {...{ sourceText, sourceLang, targetLang, setSourceText, setSourceLang, setTargetLang, hideResultWindow }} />}
      {variant === 'split' && <SplitPanelVariant {...{ sourceText, sourceLang, targetLang, setSourceText, setSourceLang, setTargetLang, hideResultWindow }} />}
      {variant === 'minimal' && <MinimalVariant {...{ sourceText, sourceLang, targetLang, setSourceText, setSourceLang, setTargetLang, hideResultWindow }} />}
      {variant === 'stack' && <CardStackVariant {...{ sourceText, sourceLang, targetLang, setSourceText, setSourceLang, setTargetLang, hideResultWindow }} />}

      {/* Variant Switcher - Bottom Bar */}
      <div className="fixed bottom-6 left-1/2 -translate-x-1/2 bg-gray-900 text-white rounded-full px-6 py-3 shadow-2xl flex items-center space-x-4 z-50">
        <span className="text-xs font-medium opacity-75">PROTOTYPE:</span>
        <button onClick={() => setVariant('bob')} className={`px-3 py-1 rounded-full text-sm transition-colors ${variant === 'bob' ? 'bg-blue-500' : 'hover:bg-gray-700'}`}>
          Bob-style
        </button>
        <button onClick={() => setVariant('split')} className={`px-3 py-1 rounded-full text-sm transition-colors ${variant === 'split' ? 'bg-blue-500' : 'hover:bg-gray-700'}`}>
          Split Panel
        </button>
        <button onClick={() => setVariant('minimal')} className={`px-3 py-1 rounded-full text-sm transition-colors ${variant === 'minimal' ? 'bg-blue-500' : 'hover:bg-gray-700'}`}>
          Minimal
        </button>
        <button onClick={() => setVariant('stack')} className={`px-3 py-1 rounded-full text-sm transition-colors ${variant === 'stack' ? 'bg-blue-500' : 'hover:bg-gray-700'}`}>
          Card Stack
        </button>
      </div>
    </div>
  );
}

// VARIATION 1: Bob-style (from UI doc)
function BobStyleVariant({ sourceText, sourceLang, targetLang, setSourceText, setSourceLang, setTargetLang, hideResultWindow }: any) {
  const [expandedCards, setExpandedCards] = useState<Set<string>>(new Set(['deeplx', 'google']));

  const toggleCard = (id: string) => {
    const newExpanded = new Set(expandedCards);
    if (newExpanded.has(id)) {
      newExpanded.delete(id);
    } else {
      newExpanded.add(id);
    }
    setExpandedCards(newExpanded);
  };

  return (
    <div className="bg-white rounded-2xl shadow-2xl w-full max-w-[640px] max-h-[85vh] overflow-hidden flex flex-col" onClick={(e) => e.stopPropagation()}>
      {/* Toolbar */}
      <div className="h-12 bg-gray-50 border-b border-gray-200 flex items-center justify-between px-4">
        <div className="flex items-center space-x-2">
          <button className="w-8 h-8 rounded hover:bg-gray-200 flex items-center justify-center transition-colors" title="固定">📌</button>
          <button className="w-8 h-8 rounded hover:bg-gray-200 flex items-center justify-center transition-colors" title="收藏">⭐</button>
          <button className="w-8 h-8 rounded hover:bg-gray-200 flex items-center justify-center transition-colors" title="历史">🕐</button>
          <button className="w-8 h-8 rounded hover:bg-gray-200 flex items-center justify-center transition-colors" title="设置">⚙️</button>
        </div>
        <button onClick={hideResultWindow} className="w-8 h-8 rounded hover:bg-gray-200 flex items-center justify-center transition-colors text-gray-500">✕</button>
      </div>

      {/* OCR Area */}
      <div className="bg-gray-50 border-b border-gray-200 p-4">
        <div className="text-sm font-medium text-gray-700 mb-2">精准识别</div>
        <textarea
          value={sourceText}
          onChange={(e) => setSourceText(e.target.value)}
          className="w-full bg-white border border-gray-300 rounded-lg px-3 py-2 text-sm resize-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
          rows={3}
          placeholder="输入或粘贴文本..."
        />
        <div className="flex items-center justify-between mt-2">
          <div className="flex items-center space-x-2">
            <button className="text-gray-500 hover:text-blue-600 text-sm">🔊</button>
            <button className="text-gray-500 hover:text-blue-600 text-sm">📋</button>
          </div>
          <span className="text-xs text-gray-500">识别为 {LANGUAGES.find(l => l.code === sourceLang)?.name}</span>
        </div>
      </div>

      {/* Language Selector */}
      <div className="bg-white border-b border-gray-200 py-4 px-6">
        <div className="flex items-center justify-center space-x-3">
          <select value={sourceLang} onChange={(e) => setSourceLang(e.target.value)} className="px-3 py-2 bg-gray-50 border border-gray-300 rounded-lg text-sm focus:ring-2 focus:ring-blue-500">
            {LANGUAGES.map((lang) => (
              <option key={lang.code} value={lang.code}>{lang.name}</option>
            ))}
          </select>
          <button className="p-2 rounded hover:bg-gray-100 disabled:opacity-50" disabled={sourceLang === 'auto'}>⇄</button>
          <select value={targetLang} onChange={(e) => setTargetLang(e.target.value)} className="px-3 py-2 bg-gray-50 border border-gray-300 rounded-lg text-sm focus:ring-2 focus:ring-blue-500">
            {LANGUAGES.filter((lang) => lang.code !== 'auto').map((lang) => (
              <option key={lang.code} value={lang.code}>{lang.name}</option>
            ))}
          </select>
        </div>
      </div>

      {/* Results */}
      <div className="flex-1 overflow-y-auto p-4 space-y-3 bg-white">
        {MOCK_TRANSLATIONS.map((trans) => (
          <div key={trans.provider_id} className="bg-gray-50 rounded-lg border border-gray-200 overflow-hidden">
            <div className="bg-white border-b border-gray-200 px-4 py-3 flex items-center justify-between cursor-pointer" onClick={() => toggleCard(trans.provider_id)}>
              <div className="flex items-center space-x-2">
                <div className="w-6 h-6 bg-blue-500 rounded text-white text-xs font-bold flex items-center justify-center">{trans.icon}</div>
                <span className="font-medium text-sm">{trans.name}</span>
              </div>
              <button className="text-gray-400 hover:text-gray-600 transition-transform" style={{ transform: expandedCards.has(trans.provider_id) ? 'rotate(180deg)' : 'rotate(0deg)' }}>∨</button>
            </div>
            {expandedCards.has(trans.provider_id) && (
              <div className="p-4">
                <p className="text-gray-700 text-sm mb-3">{trans.text}</p>
                <div className="flex items-center space-x-2">
                  <button className="text-gray-500 hover:text-blue-600 text-sm">🔊</button>
                  <button className="text-gray-500 hover:text-blue-600 text-sm">📋</button>
                </div>
              </div>
            )}
          </div>
        ))}
      </div>
    </div>
  );
}

// VARIATION 2: Split Panel
function SplitPanelVariant({ sourceText, sourceLang, targetLang, setSourceText, setSourceLang, setTargetLang, hideResultWindow }: any) {
  return (
    <div className="bg-white rounded-xl shadow-2xl w-full max-w-5xl h-[600px] flex overflow-hidden" onClick={(e) => e.stopPropagation()}>
      {/* Left Panel - Source */}
      <div className="w-1/2 bg-gradient-to-br from-blue-50 to-purple-50 p-6 flex flex-col border-r border-gray-200">
        <div className="flex items-center justify-between mb-4">
          <h3 className="font-semibold text-gray-800">源文本</h3>
          <select value={sourceLang} onChange={(e) => setSourceLang(e.target.value)} className="px-2 py-1 bg-white rounded border border-gray-300 text-sm">
            {LANGUAGES.map((lang) => (
              <option key={lang.code} value={lang.code}>{lang.name}</option>
            ))}
          </select>
        </div>
        <textarea
          value={sourceText}
          onChange={(e) => setSourceText(e.target.value)}
          className="flex-1 w-full bg-white rounded-lg p-4 border border-gray-300 resize-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
          placeholder="输入要翻译的文本..."
        />
        <div className="mt-4 flex items-center space-x-2">
          <button className="px-4 py-2 bg-white rounded-lg border border-gray-300 hover:bg-gray-50 text-sm">🔊 朗读</button>
          <button className="px-4 py-2 bg-white rounded-lg border border-gray-300 hover:bg-gray-50 text-sm">📋 复制</button>
        </div>
      </div>

      {/* Right Panel - Results */}
      <div className="w-1/2 bg-white p-6 flex flex-col">
        <div className="flex items-center justify-between mb-4">
          <h3 className="font-semibold text-gray-800">翻译结果</h3>
          <div className="flex items-center space-x-2">
            <select value={targetLang} onChange={(e) => setTargetLang(e.target.value)} className="px-2 py-1 bg-gray-50 rounded border border-gray-300 text-sm">
              {LANGUAGES.filter((lang) => lang.code !== 'auto').map((lang) => (
                <option key={lang.code} value={lang.code}>{lang.name}</option>
              ))}
            </select>
            <button onClick={hideResultWindow} className="w-8 h-8 rounded hover:bg-gray-100 flex items-center justify-center">✕</button>
          </div>
        </div>
        <div className="flex-1 overflow-y-auto space-y-3">
          {MOCK_TRANSLATIONS.map((trans) => (
            <div key={trans.provider_id} className="p-4 bg-gray-50 rounded-lg border border-gray-200">
              <div className="flex items-center space-x-2 mb-2">
                <div className="w-5 h-5 bg-gradient-to-br from-blue-500 to-purple-500 rounded text-white text-xs font-bold flex items-center justify-center">{trans.icon}</div>
                <span className="text-xs font-medium text-gray-600">{trans.name}</span>
              </div>
              <p className="text-gray-800 mb-2">{trans.text}</p>
              <div className="flex items-center space-x-2">
                <button className="text-xs text-gray-500 hover:text-blue-600">🔊</button>
                <button className="text-xs text-gray-500 hover:text-blue-600">📋</button>
              </div>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}

// VARIATION 3: Minimal
function MinimalVariant({ sourceText, sourceLang, targetLang, setSourceText, setSourceLang, setTargetLang, hideResultWindow }: any) {
  const [selectedProvider, setSelectedProvider] = useState(0);

  return (
    <div className="bg-white rounded-3xl shadow-2xl w-full max-w-2xl overflow-hidden" onClick={(e) => e.stopPropagation()}>
      {/* Minimal Header */}
      <div className="px-8 pt-6 pb-4 flex items-center justify-between">
        <div className="flex items-center space-x-3">
          {MOCK_TRANSLATIONS.map((trans, idx) => (
            <button
              key={trans.provider_id}
              onClick={() => setSelectedProvider(idx)}
              className={`w-10 h-10 rounded-full flex items-center justify-center text-sm font-bold transition-all ${
                selectedProvider === idx ? 'bg-gradient-to-br from-blue-500 to-purple-500 text-white scale-110 shadow-lg' : 'bg-gray-100 text-gray-600 hover:bg-gray-200'
              }`}
            >
              {trans.icon}
            </button>
          ))}
        </div>
        <button onClick={hideResultWindow} className="w-10 h-10 rounded-full hover:bg-gray-100 flex items-center justify-center text-gray-400">✕</button>
      </div>

      {/* Content */}
      <div className="px-8 pb-8 space-y-6">
        {/* Source */}
        <div>
          <textarea
            value={sourceText}
            onChange={(e) => setSourceText(e.target.value)}
            className="w-full bg-gray-50 rounded-2xl px-6 py-4 text-lg resize-none border-2 border-transparent focus:border-blue-500 focus:bg-white transition-all"
            rows={3}
            placeholder="输入文本..."
          />
        </div>

        {/* Language Bar */}
        <div className="flex items-center justify-center space-x-4 py-2">
          <select value={sourceLang} onChange={(e) => setSourceLang(e.target.value)} className="px-4 py-2 bg-gray-50 rounded-full text-sm border-none appearance-none cursor-pointer hover:bg-gray-100">
            {LANGUAGES.map((lang) => (
              <option key={lang.code} value={lang.code}>{lang.name}</option>
            ))}
          </select>
          <div className="w-8 h-8 rounded-full bg-gray-100 flex items-center justify-center text-sm">→</div>
          <select value={targetLang} onChange={(e) => setTargetLang(e.target.value)} className="px-4 py-2 bg-gray-50 rounded-full text-sm border-none appearance-none cursor-pointer hover:bg-gray-100">
            {LANGUAGES.filter((lang) => lang.code !== 'auto').map((lang) => (
              <option key={lang.code} value={lang.code}>{lang.name}</option>
            ))}
          </select>
        </div>

        {/* Result */}
        <div className="bg-gradient-to-br from-blue-50 to-purple-50 rounded-2xl px-6 py-6">
          <p className="text-xl text-gray-800 leading-relaxed">{MOCK_TRANSLATIONS[selectedProvider].text}</p>
          <div className="flex items-center space-x-4 mt-4 pt-4 border-t border-gray-200">
            <button className="flex items-center space-x-2 px-4 py-2 bg-white rounded-full hover:bg-gray-50 text-sm">
              <span>🔊</span>
              <span>朗读</span>
            </button>
            <button className="flex items-center space-x-2 px-4 py-2 bg-white rounded-full hover:bg-gray-50 text-sm">
              <span>📋</span>
              <span>复制</span>
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}

// VARIATION 4: Card Stack (swipeable)
function CardStackVariant({ sourceText, sourceLang, targetLang, setSourceText, setSourceLang, setTargetLang, hideResultWindow }: any) {
  const [currentCard, setCurrentCard] = useState(0);

  const nextCard = () => setCurrentCard((prev) => (prev + 1) % MOCK_TRANSLATIONS.length);
  const prevCard = () => setCurrentCard((prev) => (prev - 1 + MOCK_TRANSLATIONS.length) % MOCK_TRANSLATIONS.length);

  return (
    <div className="bg-gradient-to-br from-gray-900 to-gray-800 rounded-3xl shadow-2xl w-full max-w-md overflow-hidden" onClick={(e) => e.stopPropagation()}>
      {/* Header */}
      <div className="px-6 py-4 flex items-center justify-between border-b border-gray-700">
        <div className="flex items-center space-x-2">
          <div className="w-2 h-2 bg-green-400 rounded-full animate-pulse"></div>
          <span className="text-white text-sm font-medium">{MOCK_TRANSLATIONS[currentCard].name}</span>
        </div>
        <button onClick={hideResultWindow} className="w-8 h-8 rounded-full hover:bg-gray-700 flex items-center justify-center text-gray-400">✕</button>
      </div>

      {/* Source Input */}
      <div className="p-6 border-b border-gray-700">
        <textarea
          value={sourceText}
          onChange={(e) => setSourceText(e.target.value)}
          className="w-full bg-gray-800 text-white rounded-xl px-4 py-3 text-sm resize-none border border-gray-700 focus:border-blue-500 focus:ring-1 focus:ring-blue-500"
          rows={3}
          placeholder="输入文本..."
        />
        <div className="flex items-center justify-between mt-3">
          <select value={sourceLang} onChange={(e) => setSourceLang(e.target.value)} className="px-3 py-1 bg-gray-800 text-white rounded-lg text-xs border border-gray-700">
            {LANGUAGES.map((lang) => (
              <option key={lang.code} value={lang.code}>{lang.name}</option>
            ))}
          </select>
          <select value={targetLang} onChange={(e) => setTargetLang(e.target.value)} className="px-3 py-1 bg-gray-800 text-white rounded-lg text-xs border border-gray-700">
            {LANGUAGES.filter((lang) => lang.code !== 'auto').map((lang) => (
              <option key={lang.code} value={lang.code}>{lang.name}</option>
            ))}
          </select>
        </div>
      </div>

      {/* Card Stack */}
      <div className="p-6 min-h-[300px] flex flex-col justify-center">
        <div className="bg-gradient-to-br from-blue-500 to-purple-600 rounded-2xl p-6 shadow-xl">
          <div className="flex items-center justify-between mb-4">
            <div className="w-10 h-10 bg-white/20 backdrop-blur rounded-full flex items-center justify-center text-white font-bold">{MOCK_TRANSLATIONS[currentCard].icon}</div>
            <span className="text-xs text-white/70">{currentCard + 1} / {MOCK_TRANSLATIONS.length}</span>
          </div>
          <p className="text-white text-lg leading-relaxed mb-6">{MOCK_TRANSLATIONS[currentCard].text}</p>
          <div className="flex items-center space-x-2">
            <button className="flex-1 py-2 bg-white/20 backdrop-blur rounded-lg hover:bg-white/30 text-white text-sm">🔊 朗读</button>
            <button className="flex-1 py-2 bg-white/20 backdrop-blur rounded-lg hover:bg-white/30 text-white text-sm">📋 复制</button>
          </div>
        </div>

        {/* Navigation */}
        <div className="flex items-center justify-center space-x-4 mt-6">
          <button onClick={prevCard} className="w-10 h-10 bg-gray-800 rounded-full flex items-center justify-center text-white hover:bg-gray-700">←</button>
          <div className="flex space-x-2">
            {MOCK_TRANSLATIONS.map((_, idx) => (
              <div key={idx} className={`w-2 h-2 rounded-full transition-all ${idx === currentCard ? 'bg-blue-500 w-6' : 'bg-gray-600'}`} />
            ))}
          </div>
          <button onClick={nextCard} className="w-10 h-10 bg-gray-800 rounded-full flex items-center justify-center text-white hover:bg-gray-700">→</button>
        </div>
      </div>
    </div>
  );
}
