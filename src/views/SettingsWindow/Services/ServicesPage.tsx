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
  return <SettingsScrollPage title="服务" description="统一管理翻译、OCR 与语音服务，配置模型、接口与启用顺序"
    actions={<button type="button" className="settings-action" onClick={() => { setAddRequest((request) => request + 1); navigate({ tab: 'services', section: 'translation' }); }}>+ 添加 Provider</button>}
    requestedSectionId={requestedSection} onRequestedSectionHandled={consumeRequestedSection}
    sections={[
      { id: 'translation', label: '翻译服务', description: '拖动左侧手柄调整同组服务顺序，也可聚焦手柄后使用上下方向键', content: <TranslationProvidersPage addRequest={addRequest} showAddAction={false} /> },
      { id: 'ocr', label: 'OCR 识别引擎（单选）', description: '拖拽调整显示顺序，当前识别引擎保持单选', content: <OcrProvidersPage /> },
      { id: 'tts', label: '系统语音合成（TTS）', content: <TtsProvidersPage /> },
    ]} />;
}
