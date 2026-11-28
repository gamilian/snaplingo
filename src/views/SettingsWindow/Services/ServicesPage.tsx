import { useState } from 'react';
import { useSettingsStore } from '../../../stores/settingsStore';
import { SettingsScrollPage } from '../SettingsScrollPage';
import { OcrProvidersPage } from './OcrProvidersPage';
import { TranslationProvidersPage } from './TranslationProvidersPage';
import { TtsProvidersPage } from './TtsProvidersPage';

export function ServicesPage() {
  const [addRequest, setAddRequest] = useState(0);
  const navigate = useSettingsStore((state) => state.navigate);
  const requestedSection = useSettingsStore((state) => state.requestedSection);
  const consumeRequestedSection = useSettingsStore((state) => state.consumeRequestedSection);
  return <SettingsScrollPage title="引擎与语音" description="翻译支持多引擎并发与拖拽排序，OCR 单选切换，含系统语音合成"
    actions={<button type="button" className="settings-action" onClick={() => { setAddRequest((request) => request + 1); navigate({ tab: 'services', section: 'translation' }); }}>+ 添加 AI</button>}
    requestedSectionId={requestedSection} onRequestedSectionHandled={consumeRequestedSection}
    sections={[
      { id: 'translation', label: '翻译服务', content: <TranslationProvidersPage addRequest={addRequest} showAddAction={false} /> },
      { id: 'ocr', label: 'OCR 识别引擎（单选）', content: <OcrProvidersPage /> },
      { id: 'tts', label: '系统语音合成（TTS）', content: <TtsProvidersPage /> },
    ]} />;
}
