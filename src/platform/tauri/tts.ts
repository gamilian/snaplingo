import { invoke } from '@tauri-apps/api/core';
import type {
  SettingsTtsPort,
  SystemTtsVoice,
} from '../../application/settings/ports';
import type { ResultWindowSpeechPort } from '../../application/result-window/ports';

export const systemTts: SettingsTtsPort & ResultWindowSpeechPort = {
  listVoices() {
    return invoke<SystemTtsVoice[]>('list_system_tts_voices');
  },
  speak(text, language) {
    return invoke<void>('speak_text', { text, language });
  },
};
