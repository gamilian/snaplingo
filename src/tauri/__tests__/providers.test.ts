import { describe, expect, it, vi } from 'vitest';

const invoke = vi.fn();

vi.mock('@tauri-apps/api/core', () => ({ invoke }));

describe('providers tauri adapter', () => {
  it('activates translation provider with backend parameter name', async () => {
    const { activateTranslationProvider } = await import('../providers');
    invoke.mockResolvedValueOnce(undefined);

    await activateTranslationProvider('deepl');

    expect(invoke).toHaveBeenCalledWith('activate_translation_provider', {
      providerId: 'deepl',
    });
  });

  it('saves translation credentials as a credentials map', async () => {
    const { configureTranslationProviderCredentials } = await import('../providers');
    invoke.mockResolvedValueOnce(undefined);

    await configureTranslationProviderCredentials('baidu-translate', {
      app_id: 'app',
      secret_key: 'secret',
    });

    expect(invoke).toHaveBeenCalledWith('configure_translation_provider_credentials', {
      providerId: 'baidu-translate',
      credentials: { app_id: 'app', secret_key: 'secret' },
    });
  });

  it('loads OCR credential schema from the OCR command', async () => {
    const { getOcrProviderCredentialSchema } = await import('../providers');
    invoke.mockResolvedValueOnce([{ name: 'api_key', label: 'API Key', secret: true }]);

    await getOcrProviderCredentialSchema('baidu-ocr');

    expect(invoke).toHaveBeenCalledWith('get_ocr_provider_credential_schema', {
      providerId: 'baidu-ocr',
    });
  });

  it('saves OCR credentials as a credentials map', async () => {
    const { configureOcrProviderCredentials } = await import('../providers');
    invoke.mockResolvedValueOnce(undefined);

    await configureOcrProviderCredentials('baidu-ocr', {
      api_key: 'key',
      secret_key: 'secret',
    });

    expect(invoke).toHaveBeenCalledWith('configure_ocr_provider_credentials', {
      providerId: 'baidu-ocr',
      credentials: { api_key: 'key', secret_key: 'secret' },
    });
  });
});
