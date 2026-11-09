type CustomProviderProtocol = 'openai' | 'openai-responses' | 'anthropic' | 'gemini';

export function getCustomProviderEndpointPreview(
  protocol: CustomProviderProtocol,
  endpoint: string,
  model: string,
): string {
  switch (protocol) {
    case 'openai':
      return getOpenAICompatibleChatPreview(endpoint);
    case 'openai-responses':
      return getOpenAIResponsesPreview(endpoint);
    case 'anthropic':
      return getAnthropicMessagesPreview(endpoint);
    case 'gemini':
      return getGeminiGenerateContentPreview(endpoint, model);
  }
}

export function getOpenAICompatibleChatPreview(endpoint: string): string {
  return completeStandardEndpoint(endpoint, '/v1/chat/completions', [
    '/v1/chat/completions',
    '/v1/chat',
    '/v1',
  ]);
}

function getOpenAIResponsesPreview(endpoint: string): string {
  return completeStandardEndpoint(endpoint, '/v1/responses', [
    '/v1/responses',
    '/v1',
  ]);
}

function getAnthropicMessagesPreview(endpoint: string): string {
  return completeStandardEndpoint(endpoint, '/v1/messages', [
    '/v1/messages',
    '/v1',
  ]);
}

function getGeminiGenerateContentPreview(endpoint: string, model: string): string {
  const trimmed = endpoint.trim().replace(/\/+$/, '');
  const modelId = (model.trim() || '{model}').replace(/^models\//, '');
  const standardPath = `/v1beta/models/${modelId}:generateContent`;

  if (!trimmed) return '';
  if (isStandardGeminiGenerateContentEndpoint(trimmed)) {
    return trimmed.split('?')[0];
  }

  return completeStandardEndpoint(trimmed, standardPath, [
    standardPath,
    `/v1beta/models/${modelId}`,
    '/v1beta/models',
    '/v1beta',
  ]);
}

function completeStandardEndpoint(
  endpoint: string,
  standardPath: string,
  standardPrefixes: string[],
): string {
  const trimmed = endpoint.trim().replace(/\/+$/, '');
  if (!trimmed) return '';

  try {
    const url = new URL(trimmed);
    const path = url.pathname.replace(/\/+$/, '');
    if (!path || path === '/') {
      url.pathname = standardPath;
      return url.toString();
    }

    const prefix = findStandardEndpointSuffix(path, standardPrefixes);
    if (!prefix) return trimmed;

    url.pathname = `${path.slice(0, path.length - prefix.length)}${standardPath}`;
    return url.toString();
  } catch {
    const prefix = findStandardEndpointSuffix(trimmed, standardPrefixes);
    if (!prefix) return trimmed;

    return `${trimmed.slice(0, trimmed.length - prefix.length)}${standardPath}`;
  }
}

function findStandardEndpointSuffix(
  path: string,
  standardPrefixes: string[],
): string | null {
  let bestMatch: string | null = null;

  for (let index = 0; index < path.length; index += 1) {
    if (path[index] !== '/') continue;

    const suffix = path.slice(index);
    if (standardPrefixes.some((prefix) => prefix.startsWith(suffix))) {
      if (!bestMatch || suffix.length > bestMatch.length) {
        bestMatch = suffix;
      }
    }
  }

  return bestMatch;
}

function isStandardGeminiGenerateContentEndpoint(endpoint: string): boolean {
  const withoutQuery = endpoint.split('?')[0];
  return (
    withoutQuery.includes('/v1beta/models/') &&
    withoutQuery.endsWith(':generateContent')
  );
}
