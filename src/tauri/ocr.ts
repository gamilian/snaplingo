import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import type { OcrResult } from '../domain/capture';

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

export async function recognizeImageFile(path: string) {
  return invoke<OcrResult>('recognize_image_file', { path });
}

export async function recognizeImageData(imageData: Uint8Array | number[]) {
  return invoke<OcrResult>('recognize_image', {
    request: {
      image_data: Array.from(imageData),
      language: null,
    },
  });
}
