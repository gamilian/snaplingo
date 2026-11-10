import React from 'react';
import ReactDOM from 'react-dom/client';
import CaptureApp from './CaptureApp';
import './styles/index.css';

if (/win/i.test(navigator.platform)) {
  document.documentElement.dataset.platform = 'windows';
}

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <CaptureApp />
  </React.StrictMode>,
);
