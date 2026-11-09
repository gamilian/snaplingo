import { describe, expect, it } from 'vitest';
import {
  printableCaptureDocument,
  printBase64PngImage,
  type PrintFrameHost,
  type PrintableFrame,
} from './capturePrint';

describe('capture print adapter', () => {
  it('builds a print document around the rendered capture image', () => {
    const markup = printableCaptureDocument('rendered-png-base64');

    expect(markup).toContain('data:image/png;base64,rendered-png-base64');
    expect(markup).toContain('window.print()');
    expect(markup).toContain('window.frameElement.remove()');
  });

  it('writes the printable capture document to a hidden frame', () => {
    const writes: string[] = [];
    const calls: string[] = [];
    const frame: PrintableFrame = {
      style: {},
      contentDocument: {
        open: () => calls.push('open'),
        write: (markup) => writes.push(markup),
        close: () => calls.push('close'),
      },
      remove: () => calls.push('remove'),
    };
    const host: PrintFrameHost = {
      document: {
        createElement: (tagName) => {
          calls.push(`create:${tagName}`);
          return frame;
        },
        body: {
          appendChild: (node) => {
            calls.push('append');
            expect(node).toBe(frame);
          },
        },
      },
    };

    printBase64PngImage('rendered-png-base64', host);

    expect(frame.style).toMatchObject({
      position: 'fixed',
      width: '0',
      height: '0',
      border: '0',
    });
    expect(calls).toEqual(['create:iframe', 'append', 'open', 'close']);
    expect(writes).toEqual([printableCaptureDocument('rendered-png-base64')]);
  });

  it('reports when the print frame cannot be prepared', () => {
    const calls: string[] = [];
    const frame: PrintableFrame = {
      style: {},
      contentDocument: null,
      remove: () => calls.push('remove'),
    };
    const host: PrintFrameHost = {
      document: {
        createElement: () => frame,
        body: {
          appendChild: () => calls.push('append'),
        },
      },
    };

    expect(() => printBase64PngImage('rendered-png-base64', host)).toThrow(
      'Failed to prepare print frame',
    );
    expect(calls).toEqual(['append', 'remove']);
  });
});
