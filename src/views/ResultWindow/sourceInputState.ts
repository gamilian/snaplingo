import type {
  TranslationInputState,
} from '../../application/settings/ports';
import type { ResultWindowOrigin } from '../../application/result-window/ports';

const storageKeyPrefix = 'snaplingo.result-window.source-input';

function storageKey(origin: ResultWindowOrigin) {
  return `${storageKeyPrefix}.${origin}`;
}

export function resolveSourceInputCollapsed(
  origin: ResultWindowOrigin,
  setting: TranslationInputState | undefined,
  storage: Pick<Storage, 'getItem'> | undefined = globalThis.localStorage,
) {
  if (setting === 'collapsed') return true;
  if (setting !== 'last') return false;

  try {
    return storage?.getItem(storageKey(origin)) === 'collapsed';
  } catch {
    return false;
  }
}

export function rememberSourceInputCollapsed(
  origin: ResultWindowOrigin,
  collapsed: boolean,
  storage: Pick<Storage, 'setItem'> | undefined = globalThis.localStorage,
) {
  try {
    storage?.setItem(storageKey(origin), collapsed ? 'collapsed' : 'expanded');
  } catch {
    // Window state is a convenience preference; storage failures are non-fatal.
  }
}
