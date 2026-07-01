import { describe, expect, it } from 'vitest';
import {
  resultWindowContainerClassName,
  shouldCloseFromContainerClick,
} from './presentation';

describe('result window presentation', () => {
  it('does not render a dark modal overlay in standalone result windows', () => {
    const className = resultWindowContainerClassName('standalone');

    expect(className).not.toContain('bg-black/25');
    expect(className).not.toContain('backdrop-blur');
  });

  it('keeps the dark overlay only for the main-window embedded presentation', () => {
    expect(resultWindowContainerClassName('overlay')).toContain('bg-black/25');
  });

  it('only closes from background clicks in overlay presentation', () => {
    const target = new EventTarget();

    expect(shouldCloseFromContainerClick('overlay', target, target)).toBe(true);
    expect(shouldCloseFromContainerClick('standalone', target, target)).toBe(
      false,
    );
  });
});
