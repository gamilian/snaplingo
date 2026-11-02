export interface ScrollingCaptureView {
  id: string;
  revision: number;
  width: number;
  height: number;
  can_undo: boolean;
  preview_base64: string;
  status: 'capturing' | 'limit' | 'error' | 'completed';
  hint: string | null;
  shortcut_actions: ('copy' | 'complete' | 'undo' | 'cancel')[];
  preview_expanded: boolean;
  preview_available?: boolean;
  ui_scale?: number;
}
