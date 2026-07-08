import { describe, expect, it } from 'vitest';

import {
  createCaptureWorkspaceHostActions,
  type CaptureWorkspaceHostAdapter,
  type CaptureWorkspaceHostDeps,
} from './captureWorkspaceHost';
import {
  createInitialCaptureWorkspaceState,
  resetCaptureInteractionStatePatch,
  type CaptureWorkspaceState,
} from './captureWorkspaceState';
import {
  applyCaptureWorkspaceStatePatch,
  type CaptureWorkspaceRefs,
} from './useCaptureWorkspaceState';
import type {
  AnnotationCommand,
  CaptureSessionView,
  LogicalRect,
} from './types';

function createCaptureSessionView(
  overrides: Partial<CaptureSessionView> = {},
): CaptureSessionView {
  return {
    id: 'session-1',
    monitors: [
      {
        id: 'monitor-1',
        logical_bounds: { x: 0, y: 0, width: 500, height: 300 },
        physical_bounds: { x: 0, y: 0, width: 1000, height: 600 },
        scale_factor: 2,
        image_base64: '',
      },
    ],
    candidates: [
      {
        id: 'window-1',
        kind: 'window',
        rect: { x: 100, y: 100, width: 200, height: 120 },
        priority: 10,
      },
    ],
    captured_cursor: null,
    ...overrides,
  };
}

function createMemoryStorage() {
  const values = new Map<string, string>();

  return {
    getItem: (key: string) => values.get(key) ?? null,
    setItem: (key: string, value: string) => {
      values.set(key, value);
    },
  };
}

function createRefs(): CaptureWorkspaceRefs {
  return {
    startPointRef: { current: null },
    cursorPointRef: { current: null },
    draftSelectionRef: { current: null },
    hoverSelectionRef: { current: null },
  };
}

function createWorkspace(
  calls: string[],
  overrides: Partial<CaptureWorkspaceState> = {},
) {
  let state = applyCaptureWorkspaceStatePatch(
    createInitialCaptureWorkspaceState(),
    overrides,
  );
  const refs = createRefs();
  const adapter: CaptureWorkspaceHostAdapter = {
    refs,
    getState: () => state,
    patch: (next) => {
      recordPatch(calls, next);
      state = applyCaptureWorkspaceStatePatch(state, next);
      syncRefs(refs, next);
    },
    resetInteraction: () => {
      calls.push('reset_interaction');
      const next = resetCaptureInteractionStatePatch();
      state = applyCaptureWorkspaceStatePatch(state, next);
      refs.startPointRef.current = null;
      refs.cursorPointRef.current = null;
      refs.draftSelectionRef.current = null;
      refs.hoverSelectionRef.current = null;
    },
    resetSession: () => {
      calls.push('reset_session');
      const next = {
        status: 'idle',
        session: null,
        ...resetCaptureInteractionStatePatch(),
      } satisfies Partial<CaptureWorkspaceState>;
      state = applyCaptureWorkspaceStatePatch(state, next);
      refs.startPointRef.current = null;
      refs.cursorPointRef.current = null;
      refs.draftSelectionRef.current = null;
      refs.hoverSelectionRef.current = null;
    },
  };

  return adapter;
}

function syncRefs(
  refs: CaptureWorkspaceRefs,
  patch: Partial<CaptureWorkspaceState>,
) {
  if ('startPoint' in patch) refs.startPointRef.current = patch.startPoint ?? null;
  if ('cursorPoint' in patch) refs.cursorPointRef.current = patch.cursorPoint ?? null;
  if ('hoverSelection' in patch) {
    refs.hoverSelectionRef.current = patch.hoverSelection ?? null;
  }
}

function recordPatch(
  calls: string[],
  patch: Partial<CaptureWorkspaceState>,
) {
  const parts: string[] = [];

  if ('status' in patch) parts.push(`status:${patch.status}`);
  if ('mode' in patch) parts.push(`mode:${patch.mode}`);
  if ('session' in patch) parts.push(`session:${patch.session?.id ?? 'none'}`);
  if ('cursorPoint' in patch) {
    parts.push(`cursor:${patch.cursorPoint?.x ?? 'none'}`);
  }
  if ('hoverSelection' in patch) {
    parts.push(`hover:${formatRect(patch.hoverSelection)}`);
  }
  if ('isRenderingOutput' in patch) {
    parts.push(`rendering:${String(patch.isRenderingOutput)}`);
  }
  if ('previewImageBase64' in patch) {
    parts.push(`preview:${patch.previewImageBase64 ?? 'null'}`);
  }
  if ('error' in patch) parts.push(`error:${patch.error ?? 'null'}`);

  if (parts.length > 0) calls.push(`patch:${parts.join(',')}`);
}

function formatRect(rect: LogicalRect | null | undefined) {
  if (!rect) return 'none';
  return `${rect.x}x${rect.y}x${rect.width}x${rect.height}`;
}

function createHostHarness({
  calls,
  isCompleting = false,
  workspace,
  createCaptureSession = async () => createCaptureSessionView(),
  getCaptureSession = async () => createCaptureSessionView(),
  refreshCaptureSession = async () => createCaptureSessionView(),
  renderPreview = async () => 'preview-base64',
}: {
  calls: string[];
  isCompleting?: boolean;
  workspace: CaptureWorkspaceHostAdapter;
  createCaptureSession?: CaptureWorkspaceHostDeps['clients']['createCaptureSession'];
  getCaptureSession?: CaptureWorkspaceHostDeps['clients']['getCaptureSession'];
  refreshCaptureSession?: CaptureWorkspaceHostDeps['clients']['refreshCaptureSession'];
  renderPreview?: NonNullable<CaptureWorkspaceHostDeps['clients']['renderPreview']>;
}) {
  let completing = isCompleting;
  let cancelling = false;
  let perfState: ReturnType<CaptureWorkspaceHostDeps['getPerfState']> = null;
  let hydration: ReturnType<CaptureWorkspaceHostDeps['getSnapshotHydration']> = null;
  const storage = createMemoryStorage();
  const deps: CaptureWorkspaceHostDeps = {
    workspace,
    getScreenshotSavePath: () => '/captures',
    getOnInactive: () => undefined,
    getSelection: () => workspace.getState().selection,
    getAnnotations: () => workspace.getState().annotationHistory.annotations,
    getShouldIncludeCapturedCursor: () =>
      workspace.getState().includeCapturedCursor,
    commitTextDraftToHistory: () => {
      calls.push('commit_text_draft');
      return workspace.getState().annotationHistory;
    },
    isCompletingCapture: () => completing,
    setCompletingCapture: (value) => {
      completing = value;
      calls.push(`set_completing:${value}`);
    },
    isCancellingSession: () => cancelling,
    setCancellingSession: (value) => {
      cancelling = value;
      calls.push(`set_cancelling:${value}`);
    },
    setRevealed: (value) => {
      calls.push(`set_revealed:${value}`);
    },
    now: () => 100,
    getPerfState: () => perfState,
    setPerfState: (state) => {
      perfState = state;
      calls.push(
        state
          ? `perf_state:${state.mode}:${state.sessionId ?? 'none'}:${state.startMs}:${state.hasLoggedImagesReady}`
          : 'perf_state:none',
      );
    },
    markPerf: (event, sessionId) => {
      calls.push(`perf:${event}:${sessionId ?? 'none'}`);
    },
    storage,
    logWarning: (message) => {
      calls.push(`warn:${message}`);
    },
    getSnapshotHydration: () => hydration,
    setSnapshotHydration: (nextHydration) => {
      hydration = nextHydration;
      calls.push(`set_hydration:${nextHydration?.sessionId ?? 'none'}`);
    },
    clearHydratedSession: () => {
      calls.push('clear_hydrated');
    },
    markHydratedSession: (sessionId) => {
      calls.push(`mark_hydrated:${sessionId}`);
    },
    resetSelectionOverlay: () => {
      calls.push('reset_overlay');
    },
    resetCaptureImageReadiness: () => {
      calls.push('reset_readiness');
    },
    clients: {
      createCaptureSession,
      getCaptureSession,
      refreshCaptureSession,
      currentCaptureCursorPosition: async (sessionId) => {
        calls.push(`cursor:${sessionId}`);
        return null;
      },
      hydrateCaptureSessionSnapshots: async (sessionId) => {
        calls.push(`hydrate:${sessionId}`);
        return createCaptureSessionView({ id: sessionId });
      },
      cancelCaptureSession: async (sessionId) => {
        calls.push(`cancel_native:${sessionId}`);
      },
      renderPreview,
      runtimeEffectClient: {
        copyCaptureSelection: async () => {
          calls.push('copy_capture');
        },
      },
    },
  };

  return createCaptureWorkspaceHostActions(deps);
}

describe('captureWorkspaceHost', () => {
  it('starts sessions through loading, interaction reset, session load, application, and perf steps', async () => {
    const calls: string[] = [];
    const session = createCaptureSessionView({
      id: 'session-start',
      captured_cursor: {
        logical_position: { x: 150, y: 120 },
        hotspot: { x: 0, y: 0 },
        image_width: 16,
        image_height: 16,
        scale_factor: 2,
        image_base64: '',
      },
    });
    const workspace = createWorkspace(calls, {
      selection: { x: 1, y: 1, width: 10, height: 10 },
      error: 'old error',
      isRenderingOutput: true,
    });
    const host = createHostHarness({
      calls,
      workspace,
      createCaptureSession: async () => {
        calls.push('create_session');
        return session;
      },
    });

    const result = await host.startSession('screenshot-copy');

    expect(result?.session.id).toBe('session-start');
    expect(workspace.getState()).toMatchObject({
      status: 'selecting',
      mode: 'screenshot-copy',
      session,
      cursorPoint: { x: 150, y: 120 },
      hoverSelection: { x: 100, y: 100, width: 200, height: 120 },
      selection: null,
      error: null,
      isRenderingOutput: false,
    });
    expect(calls).toEqual([
      'set_cancelling:false',
      'set_revealed:false',
      'patch:status:loading,mode:screenshot-copy',
      'reset_interaction',
      'reset_overlay',
      'set_completing:false',
      'reset_readiness',
      'perf_state:screenshot-copy:none:100:false',
      'perf:start_session:none',
      'create_session',
      'perf_state:screenshot-copy:session-start:100:false',
      'perf:session_loaded:session-start',
      'patch:status:selecting,session:session-start,cursor:150,hover:100x100x200x120',
    ]);
  });

  it('refuses to refresh without a current session id', async () => {
    const calls: string[] = [];
    const workspace = createWorkspace(calls);
    const host = createHostHarness({
      calls,
      workspace,
      refreshCaptureSession: async () => {
        calls.push('refresh_session');
        return createCaptureSessionView();
      },
    });

    const result = await host.refreshSession();

    expect(result).toBeNull();
    expect(calls).toEqual([]);
  });

  it('guards duplicate copy completions before mutating rendering state', async () => {
    const calls: string[] = [];
    const workspace = createWorkspace(calls, {
      session: createCaptureSessionView(),
      selection: { x: 10, y: 20, width: 120, height: 80 },
      status: 'preview',
    });
    const host = createHostHarness({
      calls,
      workspace,
      isCompleting: true,
    });

    const result = await host.completePreviewSelection('copy', {
      guardCompletion: true,
    });

    expect(result).toBe(false);
    expect(calls).toEqual([]);
  });

  it('renders selection previews through rendering state, preview reset, and base64 application', async () => {
    const calls: string[] = [];
    const annotation: AnnotationCommand = {
      type: 'rectangle',
      rect: { x: 1, y: 1, width: 10, height: 10 },
      color: [255, 0, 0, 255],
      stroke_width: 2,
      filled: false,
    };
    const workspace = createWorkspace(calls, {
      session: createCaptureSessionView({ id: 'session-preview' }),
      previewImageBase64: 'old-preview',
      error: 'old error',
      includeCapturedCursor: true,
    });
    const host = createHostHarness({
      calls,
      workspace,
      renderPreview: async (input) => {
        calls.push(
          `render:${input.sessionId}:${input.annotations.length}:${String(input.includeCursor)}`,
        );
        return 'new-preview';
      },
    });

    const didRender = await host.renderSelectionPreview(
      { x: 10, y: 20, width: 120, height: 80 },
      [annotation],
      true,
    );

    expect(didRender).toBe(true);
    expect(workspace.getState()).toMatchObject({
      isRenderingOutput: false,
      previewImageBase64: 'new-preview',
      error: null,
    });
    expect(calls).toEqual([
      'patch:rendering:true',
      'patch:preview:null',
      'patch:error:null',
      'render:session-preview:1:true',
      'patch:preview:new-preview',
      'patch:rendering:false',
    ]);
  });

  it('sets error status while keeping host error messages', async () => {
    const calls: string[] = [];
    const workspace = createWorkspace(calls);
    const host = createHostHarness({
      calls,
      workspace,
      createCaptureSession: async () => {
        calls.push('create_session');
        throw new Error('load failed');
      },
    });

    const result = await host.startSession('screenshot-ocr');

    expect(result).toBeNull();
    expect(workspace.getState()).toMatchObject({
      status: 'error',
      error: 'load failed',
    });
    expect(calls).toContain('patch:status:error,error:load failed');
  });
});
