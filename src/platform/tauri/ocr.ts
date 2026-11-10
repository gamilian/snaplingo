import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import type { OcrResult } from '../../domain/capture';
import type { OcrFileResult } from '../../application/result-window/ports';

export async function selectImageFile() {
  const selected = await open({
    multiple: false,
    filters: [
      {
        name: 'Images',
        extensions: ['png', 'jpg', 'jpeg', 'webp', 'bmp', 'tif', 'tiff'],
      },
    ],
  });

  return typeof selected === 'string' ? selected : null;
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
