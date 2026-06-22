import type { OcrResult } from '../ScreenshotSession/types';

export interface OcrFileWorkflowDeps {
  selectImageFile: () => Promise<string | null>;
  recognizeImageFile: (path: string) => Promise<OcrResult>;
  setText: (text: string) => void;
  setRunning: (running: boolean) => void;
  setError: (message: string | null) => void;
}

export async function runOcrFileWorkflow(deps: OcrFileWorkflowDeps) {
  const path = await deps.selectImageFile();
  if (!path) return;

  deps.setError(null);
  deps.setRunning(true);

  try {
    const result = await deps.recognizeImageFile(path);
    deps.setText(result.text);
  } catch (err) {
    deps.setError(errorMessage(err));
  } finally {
    deps.setRunning(false);
  }
}

function errorMessage(err: unknown) {
  if (err instanceof Error) return err.message;
  return String(err);
}
