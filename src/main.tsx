import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { CAPTURE_RESULT_WINDOW_LABEL } from "./appWindowRouting";
import "./styles/index.css";

if (
  new URLSearchParams(window.location.search).get('window') ===
  CAPTURE_RESULT_WINDOW_LABEL
) {
  document.documentElement.dataset.resultWindow = 'true';
}

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
