import { invoke } from '@tauri-apps/api/core';

export interface CredentialField {
  name: string;
  label: string;
  secret: boolean;
}

export interface ProviderInfo {
  id: string;
  name: string;
  is_configured: boolean;
  requires_api_key: boolean;
  is_active: boolean;
  is_builtin: boolean;
  protocol?: string;
  endpoint?: string;
  model?: string;
  reasoning_level?: string;
}

export interface OcrProviderInfo {
  id: string;
  name: string;
  is_configured: boolean;
  requires_api_key: boolean;
  is_active: boolean;
}

export interface AddCustomTranslationProviderRequest {
  name: string;
  protocol: string;
  endpoint: string;
  model: string;
  api_key: string;
  reasoning_level?: string;
}

export function listTranslationProviders() {
  return invoke<ProviderInfo[]>('list_translation_providers');
}

export function activateTranslationProvider(providerId: string) {
  return invoke<void>('activate_translation_provider', { providerId });
}

export function deactivateTranslationProvider(providerId: string) {
  return invoke<void>('deactivate_translation_provider', { providerId });
}

export function reorderActiveTranslationProviders(providerIds: string[]) {
  return invoke<void>('reorder_active_translation_providers', { providerIds });
}

export function getProviderCredentialSchema(providerId: string) {
  return invoke<CredentialField[]>('get_provider_credential_schema', { providerId });
}

export function configureTranslationProviderCredentials(
  providerId: string,
  credentials: Record<string, string>,
) {
  return invoke<void>('configure_translation_provider_credentials', {
    providerId,
    credentials,
  });
}

export function addCustomTranslationProvider(
  request: AddCustomTranslationProviderRequest,
) {
  return invoke<ProviderInfo>('add_custom_translation_provider', { request });
}

export function removeCustomTranslationProvider(providerId: string) {
  return invoke<void>('remove_custom_translation_provider', { providerId });
}

export function listOcrProviders() {
  return invoke<OcrProviderInfo[]>('list_ocr_providers');
}

export function activateOcrProvider(providerId: string) {
  return invoke<void>('activate_ocr_provider', { providerId });
}

export function configureOcrProvider(
  providerId: string,
  apiKey: string,
  secretKey?: string,
) {
  return invoke<void>('configure_ocr_provider', {
    providerId,
    apiKey,
    secretKey: secretKey || null,
  });
}
