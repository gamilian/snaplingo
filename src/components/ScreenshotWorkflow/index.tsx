import { useState, useEffect } from 'react';
import { ScreenshotCapture } from '../ScreenshotCapture';
import { ScreenshotEditor } from '../ScreenshotEditor';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

type WorkflowState = 'idle' | 'selecting' | 'editing';

function cropDataUrl(
  imageSrc: string,
  region: { x: number; y: number; width: number; height: number },
): Promise<string> {
  return new Promise((resolve, reject) => {
    const image = new Image();

    image.onload = () => {
      const scaleX = image.naturalWidth / window.innerWidth;
      const scaleY = image.naturalHeight / window.innerHeight;
      const canvas = document.createElement('canvas');
      canvas.width = Math.round(region.width * scaleX);
      canvas.height = Math.round(region.height * scaleY);

      const context = canvas.getContext('2d');
      if (!context) {
        reject(new Error('Failed to create crop canvas'));
        return;
      }

      context.drawImage(
        image,
        Math.round(region.x * scaleX),
        Math.round(region.y * scaleY),
        canvas.width,
        canvas.height,
        0,
        0,
        canvas.width,
        canvas.height,
      );

      resolve(canvas.toDataURL('image/png'));
    };

    image.onerror = () => reject(new Error('Failed to load screenshot image'));
    image.src = imageSrc;
  });
}

export function ScreenshotWorkflow() {
  const [state, setState] = useState<WorkflowState>('idle');
  const [selectedRegion, setSelectedRegion] = useState<{
    x: number;
    y: number;
    width: number;
    height: number;
  } | null>(null);
  const [capturedImage, setCapturedImage] = useState<string | null>(null);
  const [screenImage, setScreenImage] = useState<string | null>(null);

  const startScreenshotSelection = async () => {
    try {
      const base64Data = await invoke<string>('capture_full_screen');
      setScreenImage(`data:image/png;base64,${base64Data}`);
      setCapturedImage(null);
      setSelectedRegion(null);
      setState('selecting');
    } catch (error) {
      console.error('Failed to capture screen:', error);
      alert(`截图失败: ${error}`);
      setState('idle');
    }
  };

  // 监听后端的快捷键事件（Tauri 事件）
  useEffect(() => {
    console.log('Setting up Tauri hotkey listener...');

    const unlisten = listen<string>('hotkey-triggered', (event) => {
      console.log('[ScreenshotWorkflow] Hotkey triggered from Tauri:', event.payload);

      // 只处理截图相关的快捷键
      if (event.payload === 'screenshot' ||
          event.payload === 'screenshot-ocr' ||
          event.payload === 'screenshot-translate') {
        console.log('[ScreenshotWorkflow] Starting selection...');
        startScreenshotSelection();
      }
    });

    return () => {
      console.log('Cleaning up Tauri hotkey listener...');
      unlisten.then((fn) => fn());
    };
  }, []);

  // 监听浏览器自定义事件（用于测试按钮）
  useEffect(() => {
    console.log('Setting up browser hotkey listener...');

    const handleBrowserEvent = (event: Event) => {
      const customEvent = event as CustomEvent<string>;
      console.log('[ScreenshotWorkflow] Hotkey triggered from browser:', customEvent.detail);

      if (customEvent.detail === 'screenshot' ||
          customEvent.detail === 'screenshot-ocr' ||
          customEvent.detail === 'screenshot-translate') {
        console.log('[ScreenshotWorkflow] Starting selection...');
        startScreenshotSelection();
      }
    };

    window.addEventListener('hotkey-triggered', handleBrowserEvent);
    return () => {
      console.log('Cleaning up browser hotkey listener...');
      window.removeEventListener('hotkey-triggered', handleBrowserEvent);
    };
  }, []);

  // 处理选区完成
  const handleCaptureComplete = async (region: {
    x: number;
    y: number;
    width: number;
    height: number;
  }) => {
    setSelectedRegion(region);

    try {
      if (!screenImage) {
        throw new Error('No screenshot data available');
      }

      const croppedImage = await cropDataUrl(screenImage, region);
      setCapturedImage(croppedImage);
      setState('editing');
    } catch (error) {
      console.error('Failed to capture region:', error);
      alert(`截图失败: ${error}`);
      setState('idle');
    }
  };

  // 处理取消
  const handleCancel = () => {
    setState('idle');
    setSelectedRegion(null);
    setCapturedImage(null);
    setScreenImage(null);
  };

  // 处理编辑完成
  const handleEditComplete = async (action: 'save' | 'copy' | 'pin') => {
    // TODO: 实现保存、复制、贴图功能
    console.log('Edit complete:', action);
    setState('idle');
    setSelectedRegion(null);
    setCapturedImage(null);
    setScreenImage(null);
  };

  return (
    <>
      {/* 状态1: 选区阶段（十字星光标） */}
      {state === 'selecting' && screenImage && (
        <ScreenshotCapture
          screenImage={screenImage}
          onCapture={handleCaptureComplete}
          onCancel={handleCancel}
        />
      )}

      {/* 状态2: 编辑阶段（工具栏） */}
      {state === 'editing' && capturedImage && selectedRegion && (
        <ScreenshotEditor
          image={capturedImage}
          region={selectedRegion}
          onComplete={handleEditComplete}
          onCancel={handleCancel}
        />
      )}
    </>
  );
}
