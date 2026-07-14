import type { OcrFileResult } from './ports';
export interface OcrFileWorkflowDeps {
  selectImageFile: () => Promise<string | null>;
  recognizeImageFile: (path: string, language?: string) => Promise<OcrFileResult>;
  language?: string;
  transformText: (text: string) => string;
  copyText?: (text: string) => Promise<void>;
  setText: (text: string) => void;
  setConfidence: (confidence: number | null) => void;
  setImageDataUrl: (imageDataUrl: string) => void;
  setRunning: (running: boolean) => void;
  setError: (message: string | null) => void;
}

export async function runOcrFileWorkflow(deps: OcrFileWorkflowDeps) {
  const path = await deps.selectImageFile();
  if (!path) return;

  deps.setError(null);
  deps.setRunning(true);

  try {
    const result = deps.language
      ? await deps.recognizeImageFile(path, deps.language)
      : await deps.recognizeImageFile(path);
    const text = deps.transformText(result.text);
    deps.setText(text);
    deps.setConfidence(result.confidence);
    deps.setImageDataUrl(result.imageDataUrl);
    if (deps.copyText) {
      try {
        await deps.copyText(text);
      } catch (error) {
        console.error('Failed to auto-copy OCR text:', error);
      }
    }
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
