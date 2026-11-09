import React from "react";
import ReactDOM from "react-dom/client";
import { CAPTURE_RESULT_WINDOW_LABEL } from "./appWindowRouting";
import "./styles/index.css";

if (/win/i.test(navigator.platform)) {
  document.documentElement.dataset.platform = 'windows';
}
if (['scrolling-preview', 'scrolling-outline'].includes(new URLSearchParams(window.location.search).get('window') ?? '')) {
  document.documentElement.dataset.scrollingPreview = 'true';
}
if (new URLSearchParams(window.location.search).get('window') === 'pin') {
  document.documentElement.dataset.pinnedImage = 'true';
}

const App = React.lazy(() => import('./App'));

if (
  new URLSearchParams(window.location.search).get('window') ===
  CAPTURE_RESULT_WINDOW_LABEL
) {
  document.documentElement.dataset.resultWindow = 'true';
}

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <React.Suspense fallback={null}>
      <App />
    </React.Suspense>
  </React.StrictMode>,
);
