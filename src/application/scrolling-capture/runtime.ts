import type { ScrollingCaptureView } from '../../domain/scrollingCapture';
import type { ScrollingCapturePorts, ScrollingOutputAction } from './ports';

interface ScrollingState {
  view: ScrollingCaptureView | null;
  working: boolean;
  error: string | null;
  previewExpanded: boolean;
  finished: boolean;
}

export function createScrollingCaptureRuntime(
  captureId: string,
  ports: ScrollingCapturePorts,
  options: { sampling?: boolean } = {},
) {
  let state: ScrollingState = {
    view: null,
    working: false,
    error: null,
    previewExpanded: false,
    finished: false,
  };
  let mounted = false;
  let epoch = 0;
  let lifecycleEpoch = 0;
  let timer: ReturnType<typeof setTimeout> | null = null;
  let inFlight: Promise<void> | null = null;
  let unlisteners: (() => void)[] = [];
  const listeners = new Set<() => void>();
  const patch = (next: Partial<ScrollingState>) => {
    state = { ...state, ...next };
    for (const listener of listeners) listener();
  };
  const stopTimer = () => {
    if (timer !== null) clearTimeout(timer);
    timer = null;
  };
  const apply = (view: ScrollingCaptureView) => {
    if (
      !mounted ||
      view.id !== captureId ||
      (state.view && view.revision < state.view.revision)
    )
      return;
    patch({
      view,
      error: view.status === 'error' ? view.hint : null,
      finished: view.status === 'completed',
      previewExpanded: view.preview_expanded,
    });
  };
  const queue = () => {
    stopTimer();
    if (
      !mounted ||
      options.sampling === false ||
      state.working ||
      state.error ||
      state.finished ||
      state.view?.status !== 'capturing'
    )
      return;
    timer = setTimeout(() => {
      timer = null;
      void capture();
    }, 250);
  };
  const capture = () => {
    if (inFlight || !mounted || options.sampling === false || state.working || state.finished)
      return Promise.resolve();
    const requestEpoch = epoch;
    const request = ports.commands
      .capture(captureId)
      .then((view) => {
        if (mounted && requestEpoch === epoch) apply(view);
      })
      .catch((error) => {
        if (mounted && requestEpoch === epoch) patch({ error: String(error) });
      })
      .finally(() => {
        if (inFlight === request) inFlight = null;
        if (mounted && requestEpoch === epoch) queue();
      });
    inFlight = request;
    return request;
  };
  const operation = async (
    run: () => Promise<ScrollingCaptureView | void | null>,
    finishes = false,
  ) => {
    if (!mounted || state.working || state.finished) return;
    stopTimer();
    const requestEpoch = ++epoch;
    patch({ working: true, error: null });
    try {
      await inFlight;
      if (!mounted || requestEpoch !== epoch) return;
      const view = await run();
      if (mounted && requestEpoch === epoch) {
        if (view) apply(view);
        patch({ working: false, finished: state.finished || (finishes && view !== null) });
      }
    } catch (error) {
      if (mounted && requestEpoch === epoch)
        patch({ working: false, error: String(error) });
    } finally {
      if (mounted && requestEpoch === epoch) queue();
    }
  };

  return {
    getSnapshot: () => state,
    subscribe(listener: () => void) {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    async mount() {
      if (mounted) return;
      mounted = true;
      const mountEpoch = ++epoch;
      const mountLifecycle = ++lifecycleEpoch;
      const isMounted = () => mounted && mountLifecycle === lifecycleEpoch;
      const retain = (cleanup: () => void) => {
        if (isMounted()) unlisteners.push(cleanup);
        else cleanup();
      };
      try {
        retain(
          await ports.events.subscribeUpdates((view) => {
            if (isMounted()) {
              apply(view);
              queue();
            }
          }),
        );
        if (!isMounted()) return;
        retain(
          await ports.events.subscribeErrors((error) => {
            if (isMounted()) {
              stopTimer();
              ++epoch;
              patch({ working: false, error });
            }
          }),
        );
        if (!isMounted()) return;
        const view = await ports.commands.get(captureId);
        if (mounted && mountEpoch === epoch) {
          apply(view);
          queue();
        }
      } catch (error) {
        if (mounted && mountEpoch === epoch) patch({ error: String(error) });
      }
    },
    dispose() {
      mounted = false;
      ++epoch;
      ++lifecycleEpoch;
      stopTimer();
      for (const cleanup of unlisteners) cleanup();
      unlisteners = [];
    },
    undo: () => operation(() => ports.commands.undo(captureId)),
    finish: (action: ScrollingOutputAction) =>
      operation(() => ports.commands.finish(captureId, action), true),
    save: () =>
      operation(async () => {
        const action = await ports.commands.saveAction();
        if (!action) return null;
        await ports.commands.finish(captureId, action);
      }, true),
    cancel: () => operation(() => ports.commands.cancel(captureId), true),
    resume: () => operation(() => ports.commands.capture(captureId)),
    async togglePreview() {
      if (!mounted) return;
      const requestLifecycle = lifecycleEpoch;
      const expanded = !state.previewExpanded;
      try {
        const shown = await ports.window.resizePreview(expanded);
        if (mounted && requestLifecycle === lifecycleEpoch)
          patch({ previewExpanded: shown ?? expanded });
      } catch (error) {
        if (mounted && requestLifecycle === lifecycleEpoch)
          patch({ error: String(error) });
      }
    },
    startDragging: () => ports.window.startDragging(),
  };
}

export type ScrollingCaptureRuntime = ReturnType<
  typeof createScrollingCaptureRuntime
>;
