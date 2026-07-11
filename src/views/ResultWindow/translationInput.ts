export interface InputTranslationPayload {
  text: string;
  autoTranslate: boolean;
}

export function parseInputTranslationPayload(
  payload: unknown,
): InputTranslationPayload | null {
  if (typeof payload === 'string') {
    return { text: payload, autoTranslate: false };
  }

  if (!payload || typeof payload !== 'object') return null;

  const candidate = payload as { text?: unknown; autoTranslate?: unknown };
  if (typeof candidate.text !== 'string') return null;

  return {
    text: candidate.text,
    autoTranslate: candidate.autoTranslate === true,
  };
}
