import { invoke } from '@tauri-apps/api/core';
import type { OcrResult } from '../../domain/capture';
import type { OcrFileResult } from '../../application/result-window/ports';

export function selectImageFile() {
  return invoke<string | null>('select_ocr_image_file');
}

export async function recognizeImageFile(path: string, language?: string) {
  return invoke<OcrFileResult>('recognize_image_file', {
    path,
    ...(language && language !== 'auto' ? { language } : {}),
  });
}

export async function recognizeImageData(imageData: Uint8Array | number[]) {
  return invoke<OcrResult>('recognize_image', {
    request: {
      image_data: Array.from(imageData),
      language: null,
    },
  });
}
