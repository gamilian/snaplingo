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
});
