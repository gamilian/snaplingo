import { lazy } from 'react';
import { applicationWindow } from './appWindowRouting';
import { getCurrentWindowLabel } from './platform/tauri/resultWindow';

const kind = applicationWindow(getCurrentWindowLabel(), window.location.search);
const windowApps = {
  settings: lazy(() => import('./composition/SettingsApp')),
  result: lazy(() => import('./composition/ResultApp')),
  pin: lazy(() => import('./composition/PinnedApp')),
  scrolling: lazy(() => import('./composition/ScrollingApp')),
  'scrolling-outline': lazy(() => import('./composition/ScrollingOutlineApp')),
};
const WindowApp = kind ? windowApps[kind] : null;

export default function App() {
  return WindowApp ? <WindowApp /> : null;
}
