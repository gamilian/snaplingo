import { invoke } from '@tauri-apps/api/core';
import type { AppUpdatesPort, AppUpdateStatus } from '../../application/updates/runtime';

export const appUpdates: AppUpdatesPort = {
  check: () => invoke<AppUpdateStatus>('check_app_update'),
  downloadAndOpen: () => invoke<string>('download_app_update'),
  openReleasePage: () => invoke('open_app_release_page'),
};
