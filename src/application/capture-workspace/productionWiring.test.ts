import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const runtimeView = readFileSync(
  new URL('../../views/CaptureWorkspace/useCaptureWorkspaceRuntimeView.ts', import.meta.url),
  'utf8',
);
const deletedController = new URL(
  '../../views/CaptureWorkspace/useCaptureWorkspaceController.ts',
  import.meta.url,
);
const deletedStateHook = new URL(
  '../../views/CaptureWorkspace/useCaptureWorkspaceState.ts',
  import.meta.url,
);
const captureViewRoot = readFileSync(
  new URL('../../views/CaptureWorkspace/index.tsx', import.meta.url),
  'utf8',
);
const captureView = readFileSync(
  new URL('../../views/CaptureWorkspace/CaptureWorkspaceView.tsx', import.meta.url),
  'utf8',
);
const runtime = readFileSync(new URL('../../views/CaptureWorkspace/captureWorkspaceRuntime.ts', import.meta.url), 'utf8');
const keyboard = readFileSync(
  new URL('../../views/CaptureWorkspace/captureWorkspaceKeyboard.ts', import.meta.url),
  'utf8',
);
const pointer = readFileSync(
  new URL('../../views/CaptureWorkspace/captureWorkspacePointer.ts', import.meta.url),
  'utf8',
);
const deletedEditorInput = new URL(
  '../../views/CaptureWorkspace/useCaptureWorkspaceEditorInput.ts',
  import.meta.url,
);
const deletedEditorController = new URL(
  '../../views/CaptureWorkspace/useCaptureWorkspaceEditorController.ts',
  import.meta.url,
);
const viewSources = Object.entries(
  import.meta.glob('../../views/CaptureWorkspace/*.{ts,tsx}', {
    eager: true,
    import: 'default',
    query: '?raw',
  }) as Record<string, string>,
)
  .filter(([name]) => !name.endsWith('.test.ts'))
  .map(([path, source]) => {
    const parts = path.split('/');
    return { name: parts[parts.length - 1], source };
  });

describe('capture workspace production runtime wiring', () => {
  it('routes selecting pointer and keyboard host actions through the application runtime', () => {
    expect(captureView).toContain('actions.pointerDown');
    expect(captureView).toContain('actions.pointerMove');
    expect(captureView).toContain('actions.pointerUp');
    expect(runtime).toContain('actions.keyDown(event)');
    expect(runtimeView).toContain('workflowRuntime.actions.updatePolledCursor');
    expect(runtimeView).toContain('workflowRuntime.actions.updatePolledHover');
    expect(runtimeView).not.toContain('workspace.syncHoverSelection');
  });

  it('lets the application runtime own host subscriptions and reveal lifecycle', () => {
    expect(runtimeView).toContain('workflowRuntime.actions.connectHost');
    expect(runtimeView).toContain('workflowRuntime.actions.updateHostReadiness');
    expect(runtimeView).not.toContain('hostSubscriptions');
    expect(runtimeView).not.toContain('hostWindowReveal');
    expect(runtimeView).not.toContain('keyboardHostEvents');
    expect(captureViewRoot).not.toContain('useCaptureHostSubscriptions');
    expect(captureViewRoot).not.toContain('useCaptureHostWindowReveal');
    expect(captureViewRoot).not.toContain('useCaptureKeyboardHostEvents');
    expect(runtimeView).toContain('workflowRuntime.dispose()');
    expect(runtime).toContain('dispose,');
  });

  it('does not rebuild combined host/editor pointer and keyboard action bags in the view controller', () => {
    expect(runtimeView).not.toContain('CaptureWorkspacePointerActions');
    expect(runtimeView).not.toContain('CaptureWorkspaceKeyboardActions');
    expect(runtimeView).not.toContain('pointerContext');
    expect(runtimeView).not.toContain('keyboardActions');
  });

  it('does not retain a controller-local host workflow compatibility bag', () => {
    for (const runtimeOwnedToken of [
      'const hostActions',
      'const completeCandidateSelection',
      'const completeManualSelection',
      'const resetPreviewSelection',
      'const pinSelection',
      'const selectFullCaptureArea',
      'const restoreLastSelection',
      'const restoreSelectionFromHistory',
    ]) {
      expect(runtimeView).not.toContain(runtimeOwnedToken);
    }
  });

  it('rejects renamed wide input forwarding modules anywhere in the CaptureWorkspace View', () => {
    expect(() => readFileSync(deletedEditorInput, 'utf8')).toThrow();
    expect(viewSources.map(({ name }) => name)).not.toContain(
      'useCaptureWorkspaceEditorInput.ts',
    );
    expect(
      viewSources.some(({ source }) =>
        source.includes('useCaptureWorkspaceEditorInput'),
      ),
    ).toBe(false);
    expect(
      viewSources
        .filter(({ name }) => name.startsWith('use'))
        .filter(({ source }) =>
          [
            'CaptureWorkspacePointerActions',
            'CaptureWorkspaceKeyboardActions',
            'EditorInputHost',
            'inputSetters',
          ].some((token) => source.includes(token)),
        )
        .map(({ name }) => name),
    ).toEqual([]);
  });

  it('deletes the editor controller without rebuilding an equivalent forwarding hook', () => {
    expect(() => readFileSync(deletedEditorController, 'utf8')).toThrow();
    expect(viewSources.map(({ name }) => name)).not.toContain(
      'useCaptureWorkspaceEditorController.ts',
    );
    expect(runtimeView).not.toContain('useCaptureWorkspaceEditorController');
    expect(runtimeView).not.toContain('editorController');
    expect(runtimeView).not.toContain('editorSetters');
    expect(runtimeView).not.toContain('editorInput');
    expect(
      viewSources
        .filter(({ name, source }) =>
          name.includes('EditorController') ||
          [
            'editorController',
            'editorSetters',
            'editorInput',
            'EditorInputActions',
          ].some((token) => source.includes(token)),
        )
        .map(({ name }) => name),
    ).toEqual([]);
    expect(captureView).toContain('actions.toggleAnnotationTool');
    expect(captureView).toContain('actions.commitTextDraft');
    expect(captureView).toContain('actions.resizePointerDown');
    expect(captureView).toContain('actions.wheel');
    expect(runtime).toContain('handleCaptureWorkspaceEditorKeyDown');
    expect(runtime).toContain('handleCaptureWorkspaceEditorPreviewPointerDown');
    expect(runtime).toContain('commitCaptureEditorTextDraft');
  });

  it('lets runtime terminal exclusion handle copy while editor previews are pending', () => {
    expect(runtimeView).not.toContain('isRenderingOutputRef');
    expect(runtimeView).not.toContain('guardCompletion');
    expect(runtime).toContain('interface TerminalOutputOperation');
    expect(runtime).toContain('terminalOutputOperation === operation');
    expect(runtime).not.toContain('terminalOutputInFlight');
    expect(runtime).not.toContain('if (!session || state.isRenderingOutput)');
  });

  it('deletes the wide controller and keeps the runtime View seam narrow', () => {
    expect(() => readFileSync(deletedController, 'utf8')).toThrow();
    expect(() => readFileSync(deletedStateHook, 'utf8')).toThrow();
    expect(captureViewRoot).toContain('<CaptureWorkspaceView');
    expect(captureViewRoot).toContain('renderState={renderState}');
    expect(captureViewRoot).toContain('actions={actions}');
    expect(captureViewRoot).toContain(
      'annotationColorPresets={annotationColorPresets}',
    );
    expect(captureViewRoot).toContain(
      'onUpdateAnnotationColorPresets={updateAnnotationColorPresets}',
    );
    expect(captureViewRoot).not.toContain('viewProps');
    expect(captureView).toContain('renderState,\n  actions,');
    expect(runtimeView).toContain('return { renderState, actions };');
    expect(runtimeView).not.toContain('...runtimeRenderState');
    expect(runtimeView).not.toContain('...derived');
    expect(runtimeView).not.toContain('useCaptureWorkspaceState');
    expect(captureView).not.toContain('extends CaptureWorkspaceRenderState');
    expect(captureView).not.toContain('CaptureWorkspaceDerivedState');
    expect(captureView).not.toContain('connectHost');
    expect(captureView).not.toContain('updateHostReadiness');
    expect(captureView).not.toContain('updatePolledCursor');
    expect(captureView).not.toContain('updatePolledHover');
  });

  it('keeps selecting keyboard workflows out of the editor-only handler', () => {
    const editorHandler = keyboard.slice(
      keyboard.indexOf('export function handleCaptureWorkspaceEditorKeyDown'),
      keyboard.indexOf('function isArrowKey'),
    );
    expect(editorHandler).not.toContain(
      'actions as CaptureWorkspaceKeyboardActions',
    );
    expect(editorHandler).not.toContain('handleCaptureWorkspaceKeyDown(');
    expect(editorHandler).not.toContain('planCaptureDraftSelectionKeyboardNudge');
    expect(editorHandler).not.toContain('planCaptureSelectionCursorKeyboardNudge');
    expect(editorHandler).not.toContain('planCaptureHoverSelectionCycle');
    expect(runtime).toContain('planCaptureDraftSelectionKeyboardNudge');
    expect(runtime).toContain('planCaptureSelectionCursorKeyboardNudge');
    expect(runtime).toContain('isCandidateDetectionModeToggleShortcut');
    expect(runtime).toContain('currentCaptureControlCandidate');
    expect(runtime).toContain('moveCaptureCursor');
  });

  it('deletes compatibility-only full keyboard and pointer orchestrators', () => {
    expect(keyboard).not.toContain(
      'export interface CaptureWorkspaceKeyboardActions',
    );
    expect(keyboard).not.toContain(
      'export function handleCaptureWorkspaceKeyDown',
    );
    expect(pointer).not.toContain('as CaptureWorkspacePointerActions');
    expect(pointer).not.toContain('editorPointerContext');
    expect(pointer).not.toContain(
      'export interface CaptureWorkspacePointerActions',
    );
    expect(pointer).not.toContain(
      'export function handleCaptureWorkspacePointerDown',
    );
    expect(pointer).not.toContain(
      'export function handleCaptureWorkspacePointerMove',
    );
    expect(pointer).not.toContain(
      'export function handleCaptureWorkspacePointerUp',
    );
    expect(pointer).not.toContain(
      'export function handleCaptureWorkspacePreviewPointerDown',
    );
    expect(pointer).not.toContain(
      'export function handleCaptureWorkspaceResizePointerDown',
    );
    expect(pointer).not.toContain('export function handleCaptureWorkspaceWheel');
    for (const hostOrSelectingToken of [
      'resetPreviewSelection',
      'cancelSession',
      'completeManualSelection',
      'pinSelection',
      'copySelection',
      'planCaptureDraftSelectionStart',
      'planCaptureDraftSelectionPointerMove',
      'planCaptureDraftSelectionCommit',
    ]) {
      expect(pointer).not.toContain(hostOrSelectingToken);
    }
  });
});
