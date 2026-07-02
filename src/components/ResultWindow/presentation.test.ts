import { describe, expect, it } from 'vitest';
import {
  resultWindowContainerClassName,
  shouldCloseFromContainerClick,
  shouldCloseFromWindowBlur,
} from './presentation';

describe('result window presentation', () => {
  it('does not render a dark modal overlay in standalone result windows', () => {
    const className = resultWindowContainerClassName('standalone');

    expect(className).not.toContain('bg-black/25');
    expect(className).not.toContain('backdrop-blur');
  });

  it('keeps a near-transparent hit layer for standalone blank clicks', () => {
    expect(resultWindowContainerClassName('standalone')).toContain(
      'bg-white/[0.01]',
    );
  });

  it('keeps the dark overlay only for the main-window embedded presentation', () => {
    expect(resultWindowContainerClassName('overlay')).toContain('bg-black/25');
  });

  it('closes from background clicks in overlay and standalone presentations', () => {
    const target = new EventTarget();

    expect(shouldCloseFromContainerClick('overlay', target, target)).toBe(true);
    expect(shouldCloseFromContainerClick('standalone', target, target)).toBe(
      true,
    );
  });

  it('closes standalone result windows when focus leaves the window', () => {
    expect(shouldCloseFromWindowBlur('standalone')).toBe(true);
    expect(shouldCloseFromWindowBlur('overlay')).toBe(false);
  });
});
