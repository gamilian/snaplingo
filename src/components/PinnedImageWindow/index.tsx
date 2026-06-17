import { useCallback, useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow, LogicalSize } from '@tauri-apps/api/window';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import type { PinnedImageView } from '../ScreenshotSession/types';
import {
  getPinnedContextMenuPosition,
  getPinnedDisplaySize,
  getPinnedDisplaySizeForTransform,
  getPinnedKeyboardOpacityAction,
  getPinnedKeyboardTransformAction,
  getPinnedKeyboardZoomAction,
  getPinnedOpacityFromWheel,
  getPinnedOpacityPreset,
  getPinnedTransformStyle,
  getPinnedWheelAction,
  getPinnedZoomFromWheel,
  isResetPinnedImagePointer,
  nextPinnedTransform,
} from './pinControls';
import {
  destroyPinnedImage,
  destroyPinnedImageGroup,
  hidePinnedImage,
  hidePinnedImageGroup,
  isClosePinnedImageShortcut,
  isCopyPinnedImageShortcut,
  isDestroyPinnedImageShortcut,
  isReplacePinnedImageShortcut,
  isSavePinnedImageShortcut,
  movePinnedImageToNextGroup,
  replacePinnedImageFromClipboard,
  savePinnedImage,
} from './pinActions';

const appWindow = getCurrentWindow();
const webviewWindow = getCurrentWebviewWindow();
const PIN_CONTEXT_MENU_SIZE = { width: 132, height: 332 };

function createDefaultPinnedTransform() {
  return {
    rotation: 0,
    flipX: false,
    flipY: false,
  };
}

function readPinnedImageId(search: string) {
  const params = new URLSearchParams(search);
  if (params.get('window') !== 'pin') return null;

  return params.get('imageId');
}

interface PinnedImageWindowProps {
  imageId: string;
}

export function PinnedImageWindow({ imageId }: PinnedImageWindowProps) {
  const [image, setImage] = useState<PinnedImageView | null>(null);
  const [zoom, setZoom] = useState(1);
  const [opacity, setOpacity] = useState(1);
  const [transform, setTransform] = useState(createDefaultPinnedTransform);
  const [contextMenuPosition, setContextMenuPosition] = useState<{
    x: number;
    y: number;
  } | null>(null);
  const [error, setError] = useState<string | null>(null);

  const hideCurrentPinnedImage = useCallback(async () => {
    try {
      await hidePinnedImage(webviewWindow);
    } catch (err) {
      console.error('Failed to hide pinned image:', err);
    }
  }, []);

  const destroyCurrentPinnedImage = useCallback(async () => {
    try {
      await destroyPinnedImage(invoke, imageId, webviewWindow);
    } catch (err) {
      console.error('Failed to destroy pinned image:', err);
    }
  }, [imageId]);

  const destroyCurrentPinnedImageGroup = useCallback(async () => {
    try {
      await destroyPinnedImageGroup(invoke, imageId);
      setContextMenuPosition(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }, [imageId]);

  const hideCurrentPinnedImageGroup = useCallback(async () => {
    try {
      await hidePinnedImageGroup(invoke, imageId);
      setContextMenuPosition(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }, [imageId]);

  useEffect(() => {
    let disposed = false;

    invoke<PinnedImageView>('get_pinned_image', { imageId })
      .then((nextImage) => {
        if (!disposed) {
          setImage(nextImage);
        }
      })
      .catch((err) => {
        if (!disposed) {
          setError(err instanceof Error ? err.message : String(err));
        }
      });

    return () => {
      disposed = true;
    };
  }, [imageId]);

  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key !== 'Escape') return;

      event.preventDefault();
      if (isDestroyPinnedImageShortcut(event)) {
        void destroyCurrentPinnedImage();
        return;
      }

      void hideCurrentPinnedImage();
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [destroyCurrentPinnedImage, hideCurrentPinnedImage]);

  const resizePinnedWindow = useCallback(
    async (nextZoom: number, nextTransform = transform, nextImage = image) => {
      if (!nextImage) return;

      const size = getPinnedDisplaySizeForTransform(
        nextImage,
        nextZoom,
        nextTransform,
      );
      await appWindow.setSize(new LogicalSize(size.width, size.height));
    },
    [image, transform],
  );

  const resetPinnedSize = useCallback(() => {
    setZoom(1);
    setContextMenuPosition(null);
    void resizePinnedWindow(1);
  }, [resizePinnedWindow]);

  const resetPinnedSizeAndOpacity = useCallback(() => {
    setZoom(1);
    setOpacity(1);
    setContextMenuPosition(null);
    void resizePinnedWindow(1);
  }, [resizePinnedWindow]);

  const setPinnedOpacityPreset = useCallback((nextOpacity: number) => {
    setOpacity(getPinnedOpacityPreset(nextOpacity));
    setContextMenuPosition(null);
  }, []);

  const copyPinnedImage = useCallback(async () => {
    try {
      await invoke('copy_pinned_image', { imageId });
      setContextMenuPosition(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }, [imageId]);

  const savePinnedImageAs = useCallback(async () => {
    try {
      await savePinnedImage(invoke, imageId);
      setContextMenuPosition(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }, [imageId]);

  const replacePinnedFromClipboard = useCallback(async () => {
    try {
      const nextImage = await replacePinnedImageFromClipboard<PinnedImageView>(
        invoke,
        imageId,
      );
      const nextTransform = createDefaultPinnedTransform();

      setImage(nextImage);
      setZoom(1);
      setTransform(nextTransform);
      setContextMenuPosition(null);
      await resizePinnedWindow(1, nextTransform, nextImage);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }, [imageId, resizePinnedWindow]);

  const movePinnedToAnotherGroup = useCallback(async () => {
    try {
      await movePinnedImageToNextGroup(invoke, imageId);
      setContextMenuPosition(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }, [imageId]);

  const adjustPinnedZoom = useCallback(
    (wheelDirection: number) => {
      setContextMenuPosition(null);
      setZoom((currentZoom) => {
        const nextZoom = getPinnedZoomFromWheel(currentZoom, wheelDirection);
        void resizePinnedWindow(nextZoom);
        return nextZoom;
      });
    },
    [resizePinnedWindow],
  );

  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      if (isCopyPinnedImageShortcut(event)) {
        event.preventDefault();
        void copyPinnedImage();
        return;
      }

      if (isSavePinnedImageShortcut(event)) {
        event.preventDefault();
        void savePinnedImageAs();
        return;
      }

      if (isClosePinnedImageShortcut(event)) {
        event.preventDefault();
        void hideCurrentPinnedImage();
        return;
      }

      if (isReplacePinnedImageShortcut(event)) {
        event.preventDefault();
        void replacePinnedFromClipboard();
        return;
      }

      const opacityAction = getPinnedKeyboardOpacityAction(event);
      if (opacityAction) {
        event.preventDefault();
        setContextMenuPosition(null);
        setOpacity((currentOpacity) =>
          getPinnedOpacityFromWheel(
            currentOpacity,
            opacityAction === 'increase' ? -1 : 1,
          ),
        );
        return;
      }

      const zoomAction = getPinnedKeyboardZoomAction(event);
      if (zoomAction) {
        event.preventDefault();
        if (zoomAction === 'reset') {
          resetPinnedSize();
        } else {
          adjustPinnedZoom(zoomAction === 'zoom-in' ? -1 : 1);
        }
        return;
      }

      const transformAction = getPinnedKeyboardTransformAction(event);
      if (transformAction) {
        event.preventDefault();
        setContextMenuPosition(null);
        setTransform((currentTransform) => {
          const nextTransform = nextPinnedTransform(
            currentTransform,
            transformAction,
          );
          void resizePinnedWindow(zoom, nextTransform);
          return nextTransform;
        });
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [
    adjustPinnedZoom,
    copyPinnedImage,
    hideCurrentPinnedImage,
    replacePinnedFromClipboard,
    resetPinnedSize,
    resizePinnedWindow,
    savePinnedImageAs,
    zoom,
  ]);

  const handleWheel = (event: React.WheelEvent<HTMLDivElement>) => {
    if (!image) return;

    event.preventDefault();
    setContextMenuPosition(null);
    const wheelDirection = Math.sign(event.deltaY) || 1;
    const wheelAction = getPinnedWheelAction(event);

    if (wheelAction === 'opacity') {
      setOpacity((currentOpacity) =>
        getPinnedOpacityFromWheel(currentOpacity, wheelDirection),
      );
      return;
    }

    if (wheelAction === 'zoom') {
      adjustPinnedZoom(wheelDirection);
    }
  };

  const handleContextMenu = (event: React.MouseEvent<HTMLDivElement>) => {
    event.preventDefault();
    setContextMenuPosition(
      getPinnedContextMenuPosition(
        { x: event.clientX, y: event.clientY },
        PIN_CONTEXT_MENU_SIZE,
        { width: window.innerWidth, height: window.innerHeight },
      ),
    );
  };

  if (error) {
    return (
      <div className="flex h-screen w-screen items-center justify-center bg-neutral-950 p-3 text-xs text-red-100">
        {error}
      </div>
    );
  }

  const imageFrameSize = image ? getPinnedDisplaySize(image, zoom) : null;

  return (
    <div
      className="group relative h-screen w-screen overflow-hidden bg-transparent"
      onWheel={handleWheel}
      onContextMenu={handleContextMenu}
      onPointerDown={() => setContextMenuPosition(null)}
    >
      {image && imageFrameSize && (
        <div
          className="absolute left-1/2 top-1/2"
          style={{
            width: `${imageFrameSize.width}px`,
            height: `${imageFrameSize.height}px`,
            transform: 'translate(-50%, -50%)',
          }}
        >
          <img
            src={`data:image/png;base64,${image.image_base64}`}
            className="h-full w-full object-fill"
            style={{
              opacity,
              transform: getPinnedTransformStyle(transform),
            }}
            draggable={false}
            onPointerDown={(event) => {
              if (isResetPinnedImagePointer(event)) {
                event.preventDefault();
                resetPinnedSizeAndOpacity();
                return;
              }

              if (event.button !== 0) return;
              void appWindow.startDragging();
            }}
          />
        </div>
      )}
      {contextMenuPosition && (
        <div
          className="absolute z-10 w-[132px] rounded bg-neutral-950/95 p-1 text-xs text-white shadow-xl ring-1 ring-white/15"
          style={{
            left: `${contextMenuPosition.x}px`,
            top: `${contextMenuPosition.y}px`,
          }}
          onPointerDown={(event) => event.stopPropagation()}
        >
          <button
            type="button"
            className="flex h-7 w-full items-center justify-between rounded px-2 text-left hover:bg-white/15"
            onClick={resetPinnedSize}
          >
            <span>Reset size</span>
            <span className="font-mono text-[10px] text-white/55">
              {Math.round(zoom * 100)}%
            </span>
          </button>
          {[1, 0.75, 0.5].map((preset) => (
            <button
              key={preset}
              type="button"
              className="flex h-7 w-full items-center justify-between rounded px-2 text-left hover:bg-white/15"
              onClick={() => setPinnedOpacityPreset(preset)}
            >
              <span>Opacity</span>
              <span className="font-mono text-[10px] text-white/55">
                {Math.round(preset * 100)}%
              </span>
            </button>
          ))}
          <div className="my-1 h-px bg-white/10" />
          <button
            type="button"
            className="h-7 w-full rounded px-2 text-left hover:bg-white/15"
            onClick={movePinnedToAnotherGroup}
          >
            Move Group
          </button>
          <button
            type="button"
            className="h-7 w-full rounded px-2 text-left hover:bg-white/15"
            onClick={hideCurrentPinnedImage}
          >
            Hide
          </button>
          <button
            type="button"
            className="h-7 w-full rounded px-2 text-left hover:bg-white/15"
            onClick={hideCurrentPinnedImageGroup}
          >
            Hide Group
          </button>
          <button
            type="button"
            className="h-7 w-full rounded px-2 text-left hover:bg-white/15"
            onClick={copyPinnedImage}
          >
            Copy
          </button>
          <button
            type="button"
            className="h-7 w-full rounded px-2 text-left hover:bg-white/15"
            onClick={savePinnedImageAs}
          >
            Save As
          </button>
          <button
            type="button"
            className="h-7 w-full rounded px-2 text-left text-red-100 hover:bg-red-500/20"
            onClick={destroyCurrentPinnedImage}
          >
            Destroy
          </button>
          <button
            type="button"
            className="h-7 w-full rounded px-2 text-left text-red-100 hover:bg-red-500/20"
            onClick={destroyCurrentPinnedImageGroup}
          >
            Destroy Group
          </button>
        </div>
      )}
      <button
        type="button"
        className="absolute right-1 top-1 h-6 w-6 rounded bg-black/70 text-xs leading-6 text-white opacity-0 shadow hover:bg-black/90 focus:opacity-100 group-hover:opacity-100"
        aria-label="Hide pinned image"
        title="Hide"
        onClick={hideCurrentPinnedImage}
      >
        X
      </button>
    </div>
  );
}

export function readPinnedImageLaunch(search: string) {
  return readPinnedImageId(search);
}
