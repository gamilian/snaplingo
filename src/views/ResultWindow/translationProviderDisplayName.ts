import type { Provider } from '../../stores/providerStore';

export function getTranslationProviderDisplayName(
  providerId: string,
  providers: Provider[],
): string {
  return providers.find((provider) => provider.id === providerId)?.name || providerId;
}
