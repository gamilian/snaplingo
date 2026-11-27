import { readFileSync } from 'node:fs';
import { Window } from 'happy-dom';
import { describe, expect, it } from 'vitest';

const captureEntry = readFileSync(new URL('../../capture.html', import.meta.url), 'utf8');
const applicationCss = readFileSync(new URL('./index.css', import.meta.url), 'utf8');

describe('capture window first paint', () => {
  it.each([false, true])('uses a short visual transition with reduced motion=%s', (reducedMotion) => {
    const window = new Window({ settings: { device: { prefersReducedMotion: reducedMotion ? 'reduce' : 'no-preference' } } });
    const document = window.document;
    const style = document.createElement('style');
    style.textContent = applicationCss.replace(/@(?:import|tailwind)[^;]*;/g, '');
    document.head.append(style);
    for (const attribute of ['data-capture-desktop', 'data-capture-selection-overlay']) {
      const layer = document.createElement('div');
      layer.setAttribute(attribute, '');
      layer.dataset.visible = 'false';
      document.body.append(layer);
      expect(window.getComputedStyle(layer).opacity).toBe('0');
      expect(window.getComputedStyle(layer).transition).toBe('none');
      layer.dataset.visible = 'true';
      expect(window.getComputedStyle(layer).opacity).toBe('1');
      expect(window.getComputedStyle(layer).transition).toBe(
        reducedMotion ? 'none' : 'opacity 100ms cubic-bezier(0.2, 0, 0, 1)',
      );
      layer.dataset.visible = 'false';
      expect(window.getComputedStyle(layer).opacity).toBe('0');
      expect(window.getComputedStyle(layer).transition).toBe('none');
    }
  });

  it.each(['light', 'dark'])('keeps the desktop visible before frozen images load in %s mode', (theme) => {
    const window = new Window();
    const document = window.document;
    document.documentElement.setAttribute('data-window', 'capture');
    document.documentElement.setAttribute('data-theme', theme);
    const entryStyle = document.createElement('style');
    entryStyle.textContent = captureEntry.match(/<style>([\s\S]*?)<\/style>/)![1];
    const sharedStyle = document.createElement('style');
    sharedStyle.textContent = applicationCss.replace(/@(?:import|tailwind)[^;]*;/g, '');
    // Vite appends its CSS after the entry's inline style in production.
    document.head.append(entryStyle, sharedStyle);
    const root = document.createElement('div');
    root.id = 'root';
    document.body.append(root);
    for (const surface of [document.documentElement, document.body, root]) {
      expect(window.getComputedStyle(surface).backgroundColor).toBe('transparent');
    }
    document.documentElement.removeAttribute('data-window');
    expect(window.getComputedStyle(document.body).backgroundColor).toBe(
      theme === 'dark' ? 'rgb(32 39 49)' : 'rgb(248 249 252)',
    );
  });
});
