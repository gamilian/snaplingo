import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

function App() {
  const [config, setConfig] = useState<any>(null);

  useEffect(() => {
    // Load config on mount
    invoke("get_config")
      .then((data) => setConfig(data))
      .catch((err) => console.error("Failed to load config:", err));
  }, []);

  return (
    <div className="min-h-screen bg-gray-100 flex items-center justify-center">
      <div className="bg-white p-8 rounded-lg shadow-lg max-w-2xl w-full">
        <h1 className="text-4xl font-bold text-gray-800 mb-4">
          SnapLingo
        </h1>
        <p className="text-gray-600 mb-6">
          A cross-platform screenshot, OCR, and translation tool
        </p>

        {config && (
          <div className="bg-gray-50 p-4 rounded">
            <h2 className="text-lg font-semibold mb-2">Configuration</h2>
            <pre className="text-sm text-gray-700 overflow-auto">
              {JSON.stringify(config, null, 2)}
            </pre>
          </div>
        )}

        <div className="mt-6 text-sm text-gray-500">
          <p>🚧 Project initialized - ready for development</p>
          <p className="mt-2">Next steps:</p>
          <ul className="list-disc list-inside mt-1">
            <li>Implement screenshot capture</li>
            <li>Add OCR providers</li>
            <li>Build translation system</li>
            <li>Create UI components</li>
          </ul>
        </div>
      </div>
    </div>
  );
}

export default App;
