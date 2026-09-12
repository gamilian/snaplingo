import React from "react";
import ReactDOM from "react-dom/client";
import { CAPTURE_WINDOW_LABEL } from './views/CaptureWorkspace/windowMode';
import { CAPTURE_RESULT_WINDOW_LABEL } from "./appWindowRouting";
import "./styles/index.css";

const App = React.lazy(() =>
  new URLSearchParams(window.location.search).get('window') === CAPTURE_WINDOW_LABEL
    ? import('./CaptureApp')
    : import('./App'),
);

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
