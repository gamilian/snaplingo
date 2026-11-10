import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import type { ScrollingCapturePorts } from '../../application/scrolling-capture/ports';
import type { ScrollingCaptureView } from '../../domain/scrollingCapture';
import { getSettingsSnapshot } from './settings';
import { selectCaptureSavePath } from './capture';

export const scrollingCapturePorts: ScrollingCapturePorts = {
  commands: {
    get: (captureId) => invoke('get_scrolling_capture', { captureId }),
    capture: (captureId) => invoke('capture_scrolling_frame', { captureId }),
    undo: (captureId) => invoke('undo_scrolling_capture', { captureId }),
    finish: (captureId, action) =>
      invoke('finish_scrolling_capture', { captureId, action }),
    cancel: (captureId) => invoke('cancel_scrolling_capture', { captureId }),
    async saveAction() {
      const { screenshot } = await getSettingsSnapshot();
      const path = await selectCaptureSavePath({
        directory: screenshot.savePath,
        format: screenshot.format,
        namingRule: screenshot.namingRule,
        customFileName: screenshot.customFileName,
      });
      if (!path) return null;
      return {
        type: 'save',
        path,
        format: screenshot.format,
        quality: screenshot.quality,
        copyAfterSave: screenshot.autoCopy,
        rememberChoice: true,
      };
    },
  },
  events: {
    subscribeUpdates: (handler) =>
      listen<ScrollingCaptureView>('scrolling-capture-updated', (event) =>
        handler(event.payload),
      ),
    subscribeErrors: (handler) =>
      listen<string>('scrolling-capture-error', (event) =>
        handler(event.payload),
      ),
  },
  window: {
    resizePreview: (expanded) =>
      invoke<boolean>('set_scrolling_capture_preview', { captureId: new URLSearchParams(window.location.search).get('captureId'), visible: expanded }),
    startDragging: () => getCurrentWindow().startDragging(),
  },
};
