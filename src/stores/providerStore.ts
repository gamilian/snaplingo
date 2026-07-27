import { create } from 'zustand';
import type {
  AddCustomTranslationProviderRequest,
  ProviderConfigurationState,
  SettingsConfiguration,
  UpdateCustomTranslationProviderRequest,
} from '../application/settings/configuration';

export type {
  AddCustomTranslationProviderRequest,
  Provider,
  UpdateCustomTranslationProviderRequest,
} from '../application/settings/configuration';

type ProviderConfiguration = SettingsConfiguration['providers'];

let configuration: ProviderConfiguration | null = null;
let unsubscribe: (() => void) | null = null;

export function initializeProviderStore(runtime: ProviderConfiguration) {
  unsubscribe?.();
  configuration = runtime;
  projectProviderState(runtime.getState());
  unsubscribe = runtime.subscribe(projectProviderState);
}

function runtime() {
  if (!configuration) {
    throw new Error('Provider store runtime has not been initialized');
  }
  return configuration;
}

interface ProviderState extends ProviderConfigurationState {
  loadTranslationProviders: () => Promise<void>;
  activateTranslationProvider: (id: string) => Promise<void>;
  deactivateTranslationProvider: (id: string) => Promise<void>;
  addCustomTranslationProvider: (
    request: AddCustomTranslationProviderRequest,
  ) => Promise<void>;
  updateCustomTranslationProvider: (
    id: string,
    request: UpdateCustomTranslationProviderRequest,
  ) => Promise<void>;
  removeTranslationProvider: (id: string) => Promise<void>;
  testCustomTranslationProvider: (id: string) => Promise<void>;
  loadOcrProviders: () => Promise<void>;
  activateOcrProvider: (id: string) => Promise<void>;
  configureOcrProvider: (
    providerId: string,
    credentials: Record<string, string>,
  ) => Promise<void>;
  updateProviderConfig: (
    id: string,
    providerId: string,
    config: unknown,
  ) => Promise<void>;
  reorderTranslationProviders: (ids: string[]) => Promise<void>;
}

export const useProviderStore = create<ProviderState>(() => ({
  ocrProviders: [],
  translationProviders: [],
  activeOcrProvider: null,
  activeTranslationProviders: [],
  loadTranslationProviders: async () => {
    await runtime().loadTranslation();
  },
  activateTranslationProvider: (id) => runtime().activateTranslation(id),
  deactivateTranslationProvider: (id) => runtime().deactivateTranslation(id),
  addCustomTranslationProvider: (request) =>
    runtime().addCustomTranslation(request),
  updateCustomTranslationProvider: (id, request) =>
    runtime().updateCustomTranslation(id, request),
  removeTranslationProvider: (id) => runtime().removeTranslation(id),
  testCustomTranslationProvider: (id) => runtime().testCustomTranslation(id),
  loadOcrProviders: async () => {
    await runtime().loadOcr();
  },
  activateOcrProvider: (id) => runtime().activateOcr(id),
  configureOcrProvider: (providerId, credentials) =>
    runtime().configureOcr(providerId, credentials),
  updateProviderConfig: (_id, providerId, config) =>
    runtime().configureTranslation(providerId, config),
  reorderTranslationProviders: (ids) => runtime().reorderTranslation(ids),
}));

function projectProviderState(state: ProviderConfigurationState) {
  useProviderStore.setState({
    ocrProviders: state.ocrProviders,
    translationProviders: state.translationProviders,
    activeOcrProvider: state.activeOcrProvider,
    activeTranslationProviders: state.activeTranslationProviders,
  });
}
