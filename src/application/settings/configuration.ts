import type {
  AnnotationColorPreset,
  DurableSettingsPort,
  GeneralSettings,
  HistorySettings,
  HotkeyCategory,
  HotkeySnapshot,
  OcrProviderInfo,
  OcrSettings,
  ProviderInfo,
  ScreenshotSettings,
  SettingsHotkeysPort,
  SettingsProvidersPort,
  SettingsSnapshot,
  TranslationSettings,
} from './ports';

type Unsubscribe = () => void;

export interface SettingsConfigurationEventsPort {
  subscribeSettingsChanged(handler: () => void | Promise<void>): Promise<Unsubscribe>;
  subscribeHotkeysChanged(handler: () => void | Promise<void>): Promise<Unsubscribe>;
  subscribeProvidersChanged(handler: () => void | Promise<void>): Promise<Unsubscribe>;
  subscribeHistoryChanged(handler: () => void | Promise<void>): Promise<Unsubscribe>;
  subscribeFavoritesChanged(handler: () => void | Promise<void>): Promise<Unsubscribe>;
  subscribeScreenshotFavoritesChanged(
    handler: () => void | Promise<void>,
  ): Promise<Unsubscribe>;
}

export interface Provider {
  id: string;
  name: string;
  type: 'ocr' | 'translation';
  status: 'active' | 'inactive' | 'unconfigured';
  isBuiltin: boolean;
  description?: string;
  requiresApiKey: boolean;
  config?: Record<string, unknown>;
  protocol?: string;
  endpoint?: string;
  model?: string;
  reasoningLevel?: string;
  promptStrategyId?: string;
  promptFallbackStrategyId?: string;
}

export interface AddCustomTranslationProviderRequest {
  name: string;
  protocol: string;
  endpoint: string;
  model: string;
  api_key: string;
  reasoning_level?: string;
  prompt_strategy_id?: string;
  prompt_fallback_strategy_id?: string;
}

export interface UpdateCustomTranslationProviderRequest {
  name: string;
  protocol: string;
  endpoint: string;
  model: string;
  api_key?: string;
  reasoning_level?: string;
  prompt_strategy_id?: string;
  prompt_fallback_strategy_id?: string;
}

export interface DurableSettingsConfigurationState {
  hydrated: boolean;
  snapshot: SettingsSnapshot | null;
}

export interface ProviderConfigurationState {
  ocrProviders: Provider[];
  translationProviders: Provider[];
  activeOcrProvider: string | null;
  activeTranslationProviders: string[];
}

export interface HotkeyConfigurationState {
  hydrated: boolean;
  snapshot: HotkeySnapshot | null;
  defaultSnapshot: HotkeySnapshot | null;
}

interface SettingsConfigurationPorts {
  durableSettings: DurableSettingsPort;
  providers: SettingsProvidersPort;
  hotkeys: SettingsHotkeysPort;
  events: SettingsConfigurationEventsPort;
}

interface SynchronizationOptions {
  settingsWindow: boolean;
  onSettingsChanged(snapshot: SettingsSnapshot): void | Promise<void>;
  invalidateHistory(): void;
  invalidateFavorites(): void;
  invalidateScreenshotFavorites(): void;
}

function createProjection<T>(initialState: T) {
  let state = initialState;
  const listeners = new Set<(nextState: T) => void>();

  return {
    getState: () => state,
    replace(nextState: T) {
      state = nextState;
      listeners.forEach((listener) => listener(state));
    },
    subscribe(listener: (nextState: T) => void) {
      listeners.add(listener);
      return () => {
        listeners.delete(listener);
      };
    },
  };
}

function createDurableSettingsConfiguration(port: DurableSettingsPort) {
  const projection = createProjection<DurableSettingsConfigurationState>({
    hydrated: false,
    snapshot: null,
  });
  let nextSnapshotRequest = 0;
  let latestAppliedSnapshotRequest = 0;
  let updateQueue: Promise<void> = Promise.resolve();
  let hydrationRequest: Promise<SettingsSnapshot> | null = null;

  async function applyLatestSnapshot(request: () => Promise<SettingsSnapshot>) {
    const requestId = ++nextSnapshotRequest;
    const snapshot = await request();
    if (requestId > latestAppliedSnapshotRequest) {
      latestAppliedSnapshotRequest = requestId;
      projection.replace({ hydrated: true, snapshot });
      return snapshot;
    }
    return projection.getState().snapshot ?? snapshot;
  }

  function enqueueUpdate(request: () => Promise<SettingsSnapshot>) {
    const result = updateQueue.then(request, request);
    updateQueue = result.then(
      () => undefined,
      () => undefined,
    );
    return result;
  }

  function requireSnapshot() {
    const snapshot = projection.getState().snapshot;
    if (!snapshot) throw new Error('Durable settings have not been loaded');
    return snapshot;
  }

  function hydrate() {
    const state = projection.getState();
    if (state.hydrated && state.snapshot) return Promise.resolve(state.snapshot);
    if (hydrationRequest) return hydrationRequest;

    hydrationRequest = applyLatestSnapshot(() => port.getSettingsSnapshot()).finally(
      () => {
        hydrationRequest = null;
      },
    );
    return hydrationRequest;
  }

  return {
    getState: projection.getState,
    subscribe: projection.subscribe,
    hydrate,
    refresh: () => applyLatestSnapshot(() => port.getSettingsSnapshot()),
    updateGeneral(input: Partial<GeneralSettings>) {
      return enqueueUpdate(() => {
        const current = requireSnapshot().general;
        return applyLatestSnapshot(() =>
          port.updateGeneralSettings({ ...current, ...input }),
        );
      });
    },
    updateScreenshot(input: Partial<ScreenshotSettings>) {
      return enqueueUpdate(() => {
        const current = requireSnapshot().screenshot;
        return applyLatestSnapshot(() =>
          port.updateScreenshotSettings({ ...current, ...input }),
        );
      });
    },
    updateAnnotationColors(colors: AnnotationColorPreset[]) {
      return enqueueUpdate(() =>
        applyLatestSnapshot(() => port.updateAnnotationColors(colors)),
      );
    },
    updateTranslation(input: Partial<TranslationSettings>) {
      return enqueueUpdate(() => {
        const current = requireSnapshot().translation;
        return applyLatestSnapshot(() =>
          port.updateTranslationSettings({ ...current, ...input }),
        );
      });
    },
    updateOcr(input: Partial<OcrSettings>) {
      return enqueueUpdate(() => {
        const current = requireSnapshot().ocr;
        return applyLatestSnapshot(() =>
          port.updateOcrSettings({ ...current, ...input }),
        );
      });
    },
    updateHistory(input: HistorySettings) {
      return enqueueUpdate(() =>
        applyLatestSnapshot(() => port.updateHistorySettings(input)),
      );
    },
  };
}

function providerStatus(configured: boolean, active: boolean): Provider['status'] {
  return active ? 'active' : configured ? 'inactive' : 'unconfigured';
}

function displayProviderName(info: ProviderInfo) {
  if (!info.isBuiltin && info.name.startsWith('custom-llm-') && info.model) {
    return info.model;
  }
  return info.name;
}

function toTranslationProvider(info: ProviderInfo): Provider {
  return {
    id: info.id,
    name: displayProviderName(info),
    type: 'translation',
    status: providerStatus(info.isConfigured, info.isActive),
    isBuiltin: info.isBuiltin,
    requiresApiKey: info.requiresApiKey,
    protocol: info.protocol ?? undefined,
    endpoint: info.endpoint ?? undefined,
    model: info.model ?? undefined,
    reasoningLevel: info.reasoningLevel ?? undefined,
    promptStrategyId: info.promptStrategyId ?? undefined,
    promptFallbackStrategyId: info.promptFallbackStrategyId ?? undefined,
  };
}

function toOcrProvider(info: OcrProviderInfo): Provider {
  return {
    id: info.id,
    name: info.name,
    type: 'ocr',
    status: providerStatus(info.isConfigured, info.isActive),
    isBuiltin: true,
    requiresApiKey: info.requiresApiKey,
  };
}

function normalizeTranslationCredentials(config: unknown): Record<string, string> {
  if (
    config &&
    typeof config === 'object' &&
    'apiKey' in config &&
    typeof (config as { apiKey?: unknown }).apiKey === 'string'
  ) {
    return { api_key: (config as { apiKey: string }).apiKey };
  }
  return config as Record<string, string>;
}

function createProviderConfiguration(port: SettingsProvidersPort) {
  const projection = createProjection<ProviderConfigurationState>({
    ocrProviders: [],
    translationProviders: [],
    activeOcrProvider: null,
    activeTranslationProviders: [],
  });
  let nextTranslationRequest = 0;
  let latestTranslationRequest = 0;
  let nextOcrRequest = 0;
  let latestOcrRequest = 0;

  async function loadTranslation() {
    const requestId = ++nextTranslationRequest;
    try {
      const providers = (await port.listTranslation()).map(toTranslationProvider);
      if (requestId > latestTranslationRequest) {
        latestTranslationRequest = requestId;
        projection.replace({
          ...projection.getState(),
          translationProviders: providers,
          activeTranslationProviders: providers
            .filter((provider) => provider.status === 'active')
            .map((provider) => provider.id),
        });
      }
      return projection.getState().translationProviders;
    } catch (error) {
      console.error('Failed to load translation providers:', error);
      return projection.getState().translationProviders;
    }
  }

  async function loadOcr() {
    const requestId = ++nextOcrRequest;
    try {
      const providers = (await port.listOcr()).map(toOcrProvider);
      if (requestId > latestOcrRequest) {
        latestOcrRequest = requestId;
        projection.replace({
          ...projection.getState(),
          ocrProviders: providers,
          activeOcrProvider:
            providers.find((provider) => provider.status === 'active')?.id ?? null,
        });
      }
      return projection.getState().ocrProviders;
    } catch (error) {
      console.error('Failed to load OCR providers:', error);
      return projection.getState().ocrProviders;
    }
  }

  async function mutateTranslation(operation: () => Promise<unknown>) {
    await operation();
    await loadTranslation();
  }

  async function mutateOcr(operation: () => Promise<unknown>) {
    await operation();
    await loadOcr();
  }

  return {
    getState: projection.getState,
    subscribe: projection.subscribe,
    loadTranslation,
    loadOcr,
    refresh: () => Promise.all([loadTranslation(), loadOcr()]).then(() => undefined),
    activateTranslation: (providerId: string) =>
      mutateTranslation(() => port.activateTranslation(providerId)),
    deactivateTranslation: (providerId: string) =>
      mutateTranslation(() => port.deactivateTranslation(providerId)),
    addCustomTranslation(request: AddCustomTranslationProviderRequest) {
      return mutateTranslation(() =>
        port.addCustomTranslation({
          name: request.name,
          protocol: request.protocol,
          endpoint: request.endpoint,
          model: request.model,
          apiKey: request.api_key,
          reasoningLevel: request.reasoning_level,
          promptStrategyId: request.prompt_strategy_id,
          promptFallbackStrategyId: request.prompt_fallback_strategy_id,
        }),
      );
    },
    updateCustomTranslation(
      providerId: string,
      request: UpdateCustomTranslationProviderRequest,
    ) {
      return mutateTranslation(() =>
        port.updateCustomTranslation(providerId, {
          name: request.name,
          protocol: request.protocol,
          endpoint: request.endpoint,
          model: request.model,
          apiKey: request.api_key,
          reasoningLevel: request.reasoning_level,
          promptStrategyId: request.prompt_strategy_id,
          promptFallbackStrategyId: request.prompt_fallback_strategy_id,
        }),
      );
    },
    removeTranslation: (providerId: string) =>
      mutateTranslation(() => port.removeCustomTranslation(providerId)),
    testCustomTranslation: (providerId: string) =>
      port.testCustomTranslation(providerId),
    configureTranslation(providerId: string, config: unknown) {
      return mutateTranslation(() =>
        port.configureTranslationCredentials(
          providerId,
          normalizeTranslationCredentials(config),
        ),
      );
    },
    async reorderTranslation(providerIds: string[]) {
      const activeIds = projection.getState().activeTranslationProviders;
      const reorderedActiveIds = providerIds.filter((id) => activeIds.includes(id));
      if (reorderedActiveIds.length !== activeIds.length) {
        console.warn('Reorder skipped: not all active providers included');
        return;
      }
      await mutateTranslation(() =>
        port.reorderActiveTranslation(reorderedActiveIds),
      );
    },
    activateOcr: (providerId: string) =>
      mutateOcr(() => port.activateOcr(providerId)),
    configureOcr(providerId: string, credentials: Record<string, string>) {
      return mutateOcr(() => port.configureOcrCredentials(providerId, credentials));
    },
  };
}

function cloneHotkeySnapshot(snapshot: HotkeySnapshot): HotkeySnapshot {
  return {
    screenshot: { ...snapshot.screenshot },
    translation: { ...snapshot.translation },
    ocr: { ...snapshot.ocr },
  };
}

function createHotkeyConfiguration(port: SettingsHotkeysPort) {
  const projection = createProjection<HotkeyConfigurationState>({
    hydrated: false,
    snapshot: null,
    defaultSnapshot: null,
  });
  let nextSnapshotRequest = 0;
  let latestAppliedSnapshotRequest = 0;
  let updateQueue: Promise<void> = Promise.resolve();
  let hydrationRequest: Promise<HotkeySnapshot> | null = null;

  async function applyLatestSnapshot(request: () => Promise<HotkeySnapshot>) {
    const requestId = ++nextSnapshotRequest;
    const snapshot = await request();
    if (requestId > latestAppliedSnapshotRequest) {
      latestAppliedSnapshotRequest = requestId;
      projection.replace({
        ...projection.getState(),
        hydrated: true,
        snapshot: cloneHotkeySnapshot(snapshot),
      });
      return cloneHotkeySnapshot(snapshot);
    }
    return cloneHotkeySnapshot(projection.getState().snapshot ?? snapshot);
  }

  function enqueueUpdate(request: () => Promise<HotkeySnapshot>) {
    const result = updateQueue.then(request, request);
    updateQueue = result.then(
      () => undefined,
      () => undefined,
    );
    return result;
  }

  function hydrate() {
    const state = projection.getState();
    if (state.hydrated && state.snapshot) {
      return Promise.resolve(cloneHotkeySnapshot(state.snapshot));
    }
    if (hydrationRequest) return hydrationRequest;

    const requestId = ++nextSnapshotRequest;
    hydrationRequest = Promise.all([
      port.getHotkeySnapshot(),
      port.getDefaultHotkeySnapshot(),
    ])
      .then(([snapshot, defaults]) => {
        const current = projection.getState();
        if (requestId > latestAppliedSnapshotRequest) {
          latestAppliedSnapshotRequest = requestId;
          projection.replace({
            hydrated: true,
            snapshot: cloneHotkeySnapshot(snapshot),
            defaultSnapshot: cloneHotkeySnapshot(defaults),
          });
          return cloneHotkeySnapshot(snapshot);
        }
        projection.replace({
          ...current,
          defaultSnapshot: cloneHotkeySnapshot(defaults),
        });
        return cloneHotkeySnapshot(current.snapshot ?? snapshot);
      })
      .finally(() => {
        hydrationRequest = null;
      });
    return hydrationRequest;
  }

  return {
    getState: projection.getState,
    subscribe: projection.subscribe,
    hydrate,
    refresh: () => applyLatestSnapshot(() => port.getHotkeySnapshot()),
    update(category: HotkeyCategory, action: string, hotkey: string) {
      return enqueueUpdate(() =>
        applyLatestSnapshot(() =>
          port.updateHotkey({ category, action, hotkey }).then((outcome) =>
            outcome.snapshot,
          ),
        ),
      );
    },
    reset(category: HotkeyCategory, action: string) {
      return enqueueUpdate(() =>
        applyLatestSnapshot(() =>
          port.resetHotkey(category, action).then((outcome) => outcome.snapshot),
        ),
      );
    },
    resetCategory(category: HotkeyCategory) {
      return enqueueUpdate(() =>
        applyLatestSnapshot(() => port.resetHotkeyCategory(category)),
      );
    },
  };
}

export function createSettingsConfiguration({
  durableSettings,
  providers: providerPort,
  hotkeys: hotkeyPort,
  events,
}: SettingsConfigurationPorts) {
  const settings = createDurableSettingsConfiguration(durableSettings);
  const providers = createProviderConfiguration(providerPort);
  const hotkeys = createHotkeyConfiguration(hotkeyPort);

  function synchronize(options: SynchronizationOptions) {
    let disposed = false;
    const unsubscribers: Unsubscribe[] = [];

    const track = (subscription: Promise<Unsubscribe>, label: string) => {
      subscription
        .then((unsubscribe) => {
          if (disposed) unsubscribe();
          else unsubscribers.push(unsubscribe);
        })
        .catch((error) => {
          console.warn(`Failed to subscribe to ${label} changes:`, error);
        });
    };

    track(
      events.subscribeSettingsChanged(async () => {
        try {
          const snapshot = await settings.refresh();
          await options.onSettingsChanged(snapshot);
        } catch (error) {
          console.warn('Failed to refresh durable settings:', error);
        }
      }),
      'settings',
    );
    track(
      events.subscribeProvidersChanged(() => providers.refresh()),
      'provider',
    );

    if (options.settingsWindow) {
      track(
        events.subscribeHotkeysChanged(async () => {
          try {
            await hotkeys.refresh();
          } catch (error) {
            console.warn('Failed to refresh hotkey configuration:', error);
          }
        }),
        'hotkey',
      );
      track(
        events.subscribeHistoryChanged(options.invalidateHistory),
        'history',
      );
      track(
        events.subscribeFavoritesChanged(options.invalidateFavorites),
        'favorites',
      );
      track(
        events.subscribeScreenshotFavoritesChanged(
          options.invalidateScreenshotFavorites,
        ),
        'screenshot favorites',
      );
    }

    return () => {
      disposed = true;
      unsubscribers.forEach((unsubscribe) => unsubscribe());
    };
  }

  return { settings, providers, hotkeys, synchronize };
}

export type SettingsConfiguration = ReturnType<typeof createSettingsConfiguration>;
