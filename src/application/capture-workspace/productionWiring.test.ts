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

  it('rejects renamed wide input forwarding modules anywhere in the CaptureWorkspace View', () => {
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
});
