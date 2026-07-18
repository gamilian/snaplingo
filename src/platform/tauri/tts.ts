import { invoke } from '@tauri-apps/api/core';
import type {
  SettingsTtsPort,
  SystemTtsVoice,
} from '../../application/settings/ports';

export const systemTts: SettingsTtsPort = {
  listVoices() {
    return invoke<SystemTtsVoice[]>('list_system_tts_voices');
  },
  speak(text, language) {
    return invoke<void>('speak_text', { text, language });
  },
};
