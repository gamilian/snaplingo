import { describe, expect, it } from 'vitest';
import {
  parseCaptureLaunchPayload,
  readCaptureLaunch,
} from './windowMode';

describe('capture window mode parsing', () => {
  it('treats a capture window without mode as a prewarmed idle window', () => {
    expect(readCaptureLaunch('?window=capture')).toBeNull();
  });

  it('reads an explicit capture session launch from the URL', () => {
    expect(
      readCaptureLaunch('?window=capture&mode=screenshot-ocr&sessionId=session-1'),
    ).toEqual({
      mode: 'screenshot-ocr',
      sessionId: 'session-1',
    });
    expect(
      readCaptureLaunch('?window=capture&mode=screenshot-copy&sessionId=session-2'),
    ).toEqual({
      mode: 'screenshot-copy',
      sessionId: 'session-2',
    });
  });

  it('ignores invalid capture launch modes', () => {
    expect(readCaptureLaunch('?window=capture&mode=settings')).toBeNull();
  });

  it('parses hotkey payloads for prewarmed capture windows', () => {
    expect(
      parseCaptureLaunchPayload({
        mode: 'screenshot-translate',
        sessionId: 'session-2',
      }),
    ).toEqual({
      mode: 'screenshot-translate',
      sessionId: 'session-2',
    });
  });
});
