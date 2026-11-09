// Existing React ResultWindow with isolated state and no Tauri initialization.
import { createRoot } from 'react-dom/client';
import ResultWindow from '../../../../src/views/ResultWindow/ResultWindow';
import { useResultWindowStore } from '../../../../src/stores/resultWindowStore';
import { createInitialResultWindowState } from '../../../../src/application/result-window/projection';
import { useProviderStore } from '../../../../src/stores/providerStore';
import { useSettingsConfigStore } from '../../../../src/stores/settingsConfigStore';
import type { ResultWindowRuntime } from '../../../../src/application/result-window/runtime';
import '../../../../src/styles/index.css';
import previewUrl from '../../../snaplingo-ui/tests/fixtures/library-preview.png';
const parameters = new URLSearchParams(location.search);
const mode = parameters.get('mode') === 'ocr' ? 'ocr' : 'translation';
const theme = parameters.get('theme') ?? 'native';
const appearance = parameters.get('appearance') ?? 'light';
const scale = Number(parameters.get('scale') ?? 1);
const scenario = parameters.get('scenario') ?? 'baseline';
const previewData = scenario === 'image' ? await fetch(previewUrl).then(response => response.arrayBuffer()).then(bytes => `data:image/png;base64,${btoa(String.fromCharCode(...new Uint8Array(bytes)))}`) : null;
Object.assign(document.documentElement.dataset, { interfaceTheme:theme, theme:appearance, resultWindow:'true', platform:'windows', referenceResult:`${mode}-${theme}-${appearance}-${scale}-${scenario}` });
// Scale the viewport and actual component together, as the retained native host does.
if (scale !== 1) document.documentElement.style.zoom = String(scale);
useSettingsConfigStore.setState({ hydrated:true, translation:{ autoTranslate:true, autoCopy:false, hideOnBlur:false, selectionInputState:'expanded', screenshotInputState:'expanded', windowAlwaysOnTop:true }, ocr:{ showConfidence:true } } as never);
useProviderStore.setState({ translationProviders:[{id:'google',name:'Google 翻译'},{id:'local',name:'本地翻译服务'},{id:'deeplx',name:'DeepL'}] } as never);
useResultWindowStore.setState({ ...createInitialResultWindowState(), sourceText:'The best way to predict the future is to create it.\n创造未来。', sourceLang:'auto', targetLang:'zh-CN', resultWindowVisible:true, resultWindowMode:mode,
  ocrText:scenario === 'image' ? 'SnapLingo 文字识别。\nEmail: support@example.com\n预约时间 2026-11-09 14:30' : 'SnapLingo 文字识别\n联系邮箱 support@example.com\n预约时间 2026-11-09 14:30', ocrConfidence:0.98, ocrDetectedLanguage:'zh-CN', ocrImageBase64:previewData,
  providerTranslations:[{provider_id:'google',status:'success',translated_text:'预测未来的最好方式就是创造未来。',detected_language:'en'}, {provider_id:'local',status:'pending',translated_text:''}, {provider_id:'deeplx',status:'error',translated_text:'',error:{message:'网络连接失败，请重试',retryable:true}}],
} as never, true);
const copied: string[] = [];
Object.assign(window, { resultReference:{ copied, state:() => useResultWindowStore.getState() } });
const runtime = new Proxy({}, { get:(_target,name) => name === 'getState' ? () => useResultWindowStore.getState() : name === 'copyText' ? async (text:string) => { copied.push(text); } : async () => undefined }) as ResultWindowRuntime;
createRoot(document.getElementById('root')!).render(<ResultWindow runtime={runtime} presentation="standalone"/>);
