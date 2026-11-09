// Synthetic, isolated records for comparing the actual retained Library views.
import type { SettingsRuntime } from '../../../../src/application/settings/runtime';
import type { FavoriteLibraryItem, HistoryLibraryItem } from '../../../../src/application/settings/library';
import { initializeFavoritesStore } from '../../../../src/stores/favoritesStore';
import preview from '../../../ui/tests/fixtures/library-preview.png';

export function initializeLibraryReference(): Partial<SettingsRuntime> {
  const now = Date.UTC(2026, 10, 8, 4, 0);
  Date.now = () => now;
  const date = (index: number) => new Date(now - index * 60_000).toISOString();
  const results = [
    { providerId: 'system', translatedText: '这是一段用于资料库外观验收的译文。', detectedLanguage: null, confidence: null },
    { providerId: 'other', translatedText: '第二个翻译服务的结果。', detectedLanguage: null, confidence: null },
  ];
  const translation = (id: number): HistoryLibraryItem => ({
    key: `translation:${id}`, kind: 'translation', timestamp: now - (id - 1) * 60_000,
    entry: { id, timestamp: date(id - 1), sourceText: id === 1 ? 'Library reference text.\nA second line for wrapping.' : `Saved reference ${id}`, sourceLang: 'en', targetLang: 'zh-CN', providersUsed: ['system', 'other'], results, durationMs: 120, note: null, tags: [] },
  });
  const ocr: HistoryLibraryItem = { key: 'ocr:2', kind: 'ocr', timestamp: now - 60_000,
    entry: { id: 2, timestamp: date(1), imageHash: 'fixture-image', language: null, providerUsed: 'system', recognizedText: '截图中的示例文字。\n第二行 OCR 结果。', confidence: 0.98, durationMs: 80, thumbnailDataUrl: preview, note: null, tags: [] } };
  const history: HistoryLibraryItem[] = [translation(1), ocr, ...Array.from({ length: 43 }, (_, i) => translation(i + 3))];
  const favorites: FavoriteLibraryItem[] = history.map(item => item.kind === 'translation' ? ({
    key: item.key, kind: 'translation', timestamp: item.timestamp,
    entry: { id: item.entry.id, createdAt: item.entry.timestamp, sourceHistoryId: item.entry.id, content: { contentKind: 'translation', snapshot: { sourceText: item.entry.sourceText, sourceLang: 'en', targetLang: 'zh-CN', result: { provider_id: 'system', translated_text: results[0].translatedText, detected_language: null, confidence: null } } }, note: '参考笔记\n保留第二行。', tags: ['work', '中文'], thumbnailDataUrl: null },
  }) : ({ key: item.key, kind: 'ocr', timestamp: item.timestamp,
    entry: { id: item.entry.id, createdAt: item.entry.timestamp, sourceHistoryId: item.entry.id, content: { contentKind: 'ocr', snapshot: { imageHash: 'fixture-image', recognizedText: item.entry.recognizedText, language: null, providerUsed: 'system', confidence: 0.98, sourceAssetPath: 'fixture.png', thumbnailAssetPath: 'fixture-thumb.png' } }, note: '参考笔记\n保留第二行。', tags: ['work', '中文'], thumbnailDataUrl: preview },
  }));
  favorites.splice(2, 0, { key: 'screenshot:3', kind: 'screenshot', timestamp: now - 120_000,
    entry: { id: 3, contentKind: 'screenshot', createdAt: date(2), thumbnailDataUrl: preview, width: 160, height: 90, note: '收藏截图参考', tags: ['work', '中文'] } });
  favorites.pop();
  const queries: unknown[] = []; const effects: unknown[] = [];
  const query = (items: Array<HistoryLibraryItem | FavoriteLibraryItem>, request: { filter: string; search: string; page: number; pageSize: number }) => {
    queries.push(request);
    const search = request.search.trim().toLowerCase();
    const matches = items.filter(item => (request.filter === 'all' || item.kind === request.filter) && (!search || JSON.stringify(item).toLowerCase().includes(search)));
    return { items: matches.slice(request.page * request.pageSize, (request.page + 1) * request.pageSize), total: matches.length };
  };
  const library = {
    queryHistory: async (request: never) => ({ ...query(history, request), favoriteKeys: new Set([JSON.stringify(['translation', history[0].kind === 'translation' ? history[0].entry.sourceText : '', 'en', 'zh-CN', 'other', results[1].translatedText])]) }),
    queryFavorites: async (request: never) => query(favorites, request),
    deleteHistory: async (item: HistoryLibraryItem) => { history.splice(history.indexOf(item), 1); },
    clearHistory: async (filter: string) => { for (let i=history.length-1;i>=0;i--) if(filter==='all'||history[i].kind===filter) history.splice(i,1); },
    deleteFavorite: async (item: FavoriteLibraryItem) => { favorites.splice(favorites.indexOf(item), 1); },
    updateFavoriteMetadata: async (item: FavoriteLibraryItem, note: string | null, tags: string[]) => { item.entry.note=note;item.entry.tags=tags;effects.push({note,tags}); },
    rerunHistoryOcrAndCopy: async (id: number) => { effects.push({ rerunHistory: id }); },
    rerunFavoriteOcrAndCopy: async (id: number) => { effects.push({ rerunFavorite: id }); },
  };
  const runtime = { library, favorites: { addTranslation: async () => 1, addOcr: async () => 2 }, screenshotFavorites: { copy: async(id:number)=>{effects.push({copyImage:id});}, reveal:async(id:number)=>{effects.push({reveal:id});} }, clipboard:{copyText:async(text:string)=>{effects.push({copy:text});}} } as unknown as Partial<SettingsRuntime>;
  initializeFavoritesStore(runtime.favorites!);
  Object.assign(window,{libraryReference:{snapshot:()=>({queries,effects}),select:(index:number)=>{(document.querySelectorAll('.library-list-item')[index] as HTMLButtonElement)?.click();}}});
  return runtime;
}
