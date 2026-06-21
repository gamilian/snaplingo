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
});
