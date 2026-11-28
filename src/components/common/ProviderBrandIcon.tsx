import google from '../../assets/providers/google.svg';
import deepl from '../../assets/providers/deepl.svg';
import openai from '../../assets/providers/openai.svg';
import claude from '../../assets/providers/claude.svg';
import baidu from '../../assets/providers/baidu.svg';
import apple from '../../assets/providers/apple.svg';
import paddle from '../../assets/providers/paddle.svg';
import gemini from '../../assets/providers/gemini.svg';

interface ProviderIdentity {
  id: string;
  name?: string;
  protocol?: string;
  endpoint?: string;
}

export function ProviderBrandIcon({ id, name, protocol, endpoint, small = false }: ProviderIdentity & { small?: boolean }) {
  const identity = `${id} ${name ?? ''} ${endpoint ?? ''}`.toLowerCase();
  const system = id === 'system-ocr' || id === 'system-tts';
  const isApple = system && /Mac|iPhone|iPad/i.test(navigator.platform);
  let icon: string | undefined;
  let background = 'rgb(var(--sunken))';
  if (identity.includes('google') && !identity.includes('gemini')) { icon = google; background = '#fff'; }
  else if (identity.includes('deepl')) { icon = deepl; background = '#0f2b46'; }
  else if (identity.includes('claude') || identity.includes('anthropic') || protocol === 'anthropic') { icon = claude; background = '#d97757'; }
  else if (identity.includes('gemini') || protocol === 'gemini') { icon = gemini; background = '#4285f4'; }
  else if (identity.includes('openai') || /\bgpt[-\s]/.test(identity)) { icon = openai; background = '#10a37f'; }
  else if (identity.includes('baidu') || identity.includes('百度')) { icon = baidu; background = '#2932e1'; }
  else if (identity.includes('paddle')) { icon = paddle; background = '#2932e1'; }
  else if (isApple) { icon = apple; background = '#111827'; }

  return <span className={`provider-brand${small ? ' provider-brand-small' : ''}`} style={{ background }} aria-hidden="true">
    {icon ? <img src={icon} alt="" /> : <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.7" className="text-muted">
      {system ? <path d="M3 3h8v8H3zM14 3h7v8h-7zM3 14h8v7H3zM14 14h7v7h-7z" /> : <><path d="m12 3 8 4.5v9L12 21l-8-4.5v-9L12 3Z" /><path d="m12 7 4 2.5v5L12 17l-4-2.5v-5L12 7Z" /></>}
    </svg>}
  </span>;
}
