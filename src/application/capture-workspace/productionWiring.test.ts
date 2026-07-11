import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const controller = readFileSync(
  new URL('../../views/CaptureWorkspace/useCaptureWorkspaceController.ts', import.meta.url),
  'utf8',
);
const captureViewRoot = readFileSync(
  new URL('../../views/CaptureWorkspace/index.tsx', import.meta.url),
  'utf8',
);
const runtime = readFileSync(new URL('./runtime.ts', import.meta.url), 'utf8');
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
    expect(controller).toContain('workflowRuntime.actions.pointerDown');
    expect(controller).toContain('workflowRuntime.actions.pointerMove');
    expect(controller).toContain('workflowRuntime.actions.pointerUp');
    expect(runtime).toContain('actions.keyDown(keyboardEvent)');
  });

  it('lets the application runtime own host subscriptions and reveal lifecycle', () => {
    expect(controller).toContain('workflowRuntime.actions.connectHost');
    expect(controller).toContain('workflowRuntime.actions.updateHostReadiness');
    expect(controller).not.toContain('hostSubscriptions');
    expect(controller).not.toContain('hostWindowReveal');
    expect(controller).not.toContain('keyboardHostEvents');
    expect(captureViewRoot).not.toContain('useCaptureHostSubscriptions');
    expect(captureViewRoot).not.toContain('useCaptureHostWindowReveal');
    expect(captureViewRoot).not.toContain('useCaptureKeyboardHostEvents');
  });

  it('does not rebuild combined host/editor pointer and keyboard action bags in the view controller', () => {
    expect(controller).not.toContain('CaptureWorkspacePointerActions');
    expect(controller).not.toContain('CaptureWorkspaceKeyboardActions');
    expect(controller).not.toContain('pointerContext');
    expect(controller).not.toContain('keyboardActions');
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
      expect(controller).not.toContain(runtimeOwnedToken);
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
    expect(runtime).toContain('planCaptureHoverSelectionCycle');
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
