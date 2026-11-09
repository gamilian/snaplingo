// Isolated stateful ports for the actual OCR/Translation React views. Recording
// and edits exercise their stores without touching OS registrations or settings.
import { useSettingsConfigStore } from '../../../../src/stores/settingsConfigStore';
import { useHotkeyConfigStore } from '../../../../src/stores/hotkeyConfigStore';
import type { HotkeyCategory, HotkeySnapshot } from '../../../../src/application/settings/ports';

export function initializeFeatureReference() {
  const defaults: HotkeySnapshot = {
    screenshot: { screenshot: 'F1', 'screenshot-copy': '⌘F1', pin: 'F3', 'pin-toggle-all': '⇧F3', 'pin-switch-group': '⌘F3' },
    translation: { 'selection-translate': '⌥D', 'screenshot-translate': '⌥S', 'show-window': '⌥A' },
    ocr: { 'screenshot-ocr': '⇧⌥S', 'silent-screenshot-ocr': '未设置', 'file-ocr': '未设置' },
  };
  let snapshot = structuredClone(defaults);
  let recordingId: string | null = null;
  const recorded = new Set<(event: { recordingId: string; hotkey: string }) => void>();
  const alerts: string[] = [];
  window.alert = (message) => { alerts.push(String(message)); };
  window.confirm = () => true;

  const update = async (category: HotkeyCategory, action: string, hotkey: string) => {
    if (hotkey !== '未设置' && Object.values(snapshot).some((group) => Object.entries(group).some(([key, value]) => value === hotkey && (group !== snapshot[category] || key !== action)))) {
      throw new Error('该快捷键已被其他操作使用');
    }
    snapshot = { ...snapshot, [category]: { ...snapshot[category], [action]: hotkey } };
    useHotkeyConfigStore.setState({ snapshot: structuredClone(snapshot) });
    return structuredClone(snapshot);
  };
  useHotkeyConfigStore.setState({
    hydrated: true, snapshot, defaultSnapshot: structuredClone(defaults),
    beginRecording: async (id) => { recordingId = id; },
    endRecording: async (id) => { if (recordingId === id) recordingId = null; },
    subscribeRecordedHotkey: async (receive) => { recorded.add(receive); return () => { recorded.delete(receive); }; },
    updateHotkey: update,
    resetHotkey: (category, action) => update(category, action, defaults[category][action]),
    resetCategory: async (category) => {
      snapshot = { ...snapshot, [category]: { ...defaults[category] } };
      useHotkeyConfigStore.setState({ snapshot: structuredClone(snapshot) });
      return structuredClone(snapshot);
    },
  });
  useSettingsConfigStore.setState({
    screenshot: {savePath:'~/Pictures/SnapLingo',format:'png',quality:90,namingRule:'timestamp',customFileName:'SnapLingo',autoCopy:false,defaultStrokeWidth:2,defaultFontSize:24,rememberLastTool:true,showSelectionSize:true,showMagnifier:false,magnifierZoom:12,pinOpacity:100,pinShadow:true,pinBorder:false,pinPosition:'original',pinBorderShortcut:'b',longshotMaxHeight:30000,captureUiScale:'auto',annotationToolShortcuts:{rectangle:'b',ellipse:'o',arrow:'a',line:'l',pen:'p',highlight:'h','region-highlight':'q',blur:'u',spotlight:'g',mosaic:'m',text:'t',eraser:'e',scrolling:'s'},annotationColors:[[255,77,79,255]],selectionBorderWidth:2,selectionBorderColor:[91,127,255,242],selectionMaskColor:[0,0,0,46]},
    updateScreenshotSettings:async(input)=>{useSettingsConfigStore.setState(state=>({screenshot:{...state.screenshot!,...input}}));return durableSnapshot();},
    ocr: { gpuAcceleration: true, recognitionLanguage: 'auto', preserveFormatting: true, removeChineseSpaces: true, showConfidence: false, windowPosition: 'cursor', hideSilentStatus: false },
    translation: {
      defaultSourceLang: 'auto', defaultTargetLang: 'zh-CN', autoTranslate: true, autoCopy: false,
      preserveLineBreaks: true, incrementalTranslation: false, windowAlwaysOnTop: true, hideOnBlur: true,
      selectionWindowPosition: 'below-cursor', inputWindowPosition: 'center', selectionInputState: 'last',
      screenshotInputState: 'last', maxWindowHeightRatio: 70, windowWidth: 660, selectionTextMode: 'smart',
    },
    updateOcrSettings: async (input) => {
      useSettingsConfigStore.setState((state) => ({ ocr: { ...state.ocr!, ...input } }));
      return durableSnapshot();
    },
    updateTranslationSettings: async (input) => {
      useSettingsConfigStore.setState((state) => ({ translation: { ...state.translation!, ...input } }));
      return durableSnapshot();
    },
  });
  Object.assign(window, { settingsFeatureReference: {
    snapshot: () => ({ hotkeys: snapshot, ocr: useSettingsConfigStore.getState().ocr, translation: useSettingsConfigStore.getState().translation, recordingId, listeners: recorded.size, alerts }),
    emit: (hotkey: string) => { if (recordingId) for (const receive of recorded) receive({ recordingId, hotkey }); },
  } });
}

function durableSnapshot() {
  const state = useSettingsConfigStore.getState();
  return { general: state.general!, screenshot: state.screenshot!, translation: state.translation!, ocr: state.ocr!, history: state.history! };
}
