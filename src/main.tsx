import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import ResultWindow from "./components/ResultWindow";
import "./styles/index.css";

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <App />
    <ResultWindow />
  </React.StrictMode>,
);
