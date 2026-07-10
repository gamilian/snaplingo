import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import {
  createLogicalSize,
  createPhysicalPosition,
  getCurrentAppWebviewWindow,
  getCurrentAppWindow,
  getWebviewWindowByLabel,
} from '../../tauri/window';
import { writeClipboardText } from '../../tauri/clipboard';
import { useSettingsConfigStore } from '../../stores/settingsConfigStore';
import { getPinnedImage } from '../../tauri/pinnedImage';
import type { PinnedImageView } from '../../domain/capture';
import {
  getPinnedContextMenuPosition,
  getPinnedDisplaySize,
  getPinnedDisplaySizeForTransform,
  getPinnedImagePointFromPointer,
  getPinnedKeyboardMoveDelta,
  getPinnedKeyboardOpacityAction,
  getPinnedKeyboardToolbarAction,
  getPinnedKeyboardTransformAction,
  getPinnedKeyboardVisualFilterAction,
  getPinnedKeyboardZoomAction,
  getPinnedOpacityFromWheel,
  getPinnedOpacityPreset,
  getPinnedThumbnailDisplaySize,
  getPinnedTransformStyle,
  getPinnedVisualFilterStyle,
  getPinnedWheelAction,
  getPinnedZoomFromWheel,
  isClosePinnedImageDoubleClick,
  isPinnedMagnifierShortcut,
  isResetPinnedImagePointer,
  isTogglePinnedThumbnailModeDoubleClick,
  nextPinnedTransform,
  nextPinnedVisualFilter,
} from './pinControls';
import {
  colorSampleToClipboardText,
  isColorSampleCopyShortcut,
  isColorSampleFormatToggleShortcut,
  sampleCanvasColor,
  type ColorSample,
  type ColorSampleFormat,
} from '../ScreenshotSession/colorSampler';
import {
  getMagnifierImageStyle,
  getMagnifierPosition,
} from '../ScreenshotSession/magnifier';
import {
  copyPinnedImage,
  copyPinnedText,
  closePinnedImage,
  destroyPinnedImage,
  destroyPinnedImageGroup,
  getPinnedHoverToolbarActions,
  hidePinnedImageGroup,
  isClosePinnedImageShortcut,
  isCopyPinnedImageShortcut,
  isCopyPinnedTextShortcut,
  isDestroyPinnedImageShortcut,
  isOpenPinnedPreferencesShortcut,
  isQuickSavePinnedImageShortcut,
  isReplacePinnedImageShortcut,
  isSavePinnedImageShortcut,
  movePinnedImageToNextGroup,
  openPinnedPreferences,
  quickSavePinnedImage,
  replacePinnedImageFromClipboard,
  savePinnedImage,
} from './pinActions';

const appWindow = getCurrentAppWindow();
const webviewWindow = getCurrentAppWebviewWindow();
const PIN_CONTEXT_MENU_SIZE = { width: 132, height: 332 };
const PIN_HOVER_TOOLBAR_ACTIONS = getPinnedHoverToolbarActions();
const PIN_MAGNIFIER_SIZE = { width: 160, height: 112 };
const PIN_MAGNIFIER_GAP = 14;
const PIN_MAGNIFIER_ZOOM = 8;

function createDefaultPinnedTransform() {
  return {
    rotation: 0,
    flipX: false,
    flipY: false,
  };
}

function createDefaultPinnedVisualFilter() {
  return {
    grayscale: false,
    inverted: false,
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
  const screenshotSavePath = useSettingsConfigStore(
    (state) => state.screenshot?.savePath,
  );
  const [image, setImage] = useState<PinnedImageView | null>(null);
  const [zoom, setZoom] = useState(1);
  const [opacity, setOpacity] = useState(1);
  const [isThumbnailMode, setIsThumbnailMode] = useState(false);
  const [transform, setTransform] = useState(createDefaultPinnedTransform);
  const [visualFilter, setVisualFilter] = useState(createDefaultPinnedVisualFilter);
  const [contextMenuPosition, setContextMenuPosition] = useState<{
    x: number;
    y: number;
  } | null>(null);
  const imageFrameRef = useRef<HTMLDivElement | null>(null);
  const sampleCanvasRef = useRef<HTMLCanvasElement | null>(null);
  const [imagePointerPoint, setImagePointerPoint] = useState<{
    x: number;
    y: number;
  } | null>(null);
  const [viewportPointerPoint, setViewportPointerPoint] = useState<{
    x: number;
    y: number;
  } | null>(null);
  const [cursorColor, setCursorColor] = useState<ColorSample | null>(null);
  const [colorSampleFormat, setColorSampleFormat] =
    useState<ColorSampleFormat>('hex');
  const [isMagnifierRequested, setIsMagnifierRequested] = useState(false);
  const [sampleCanvasVersion, setSampleCanvasVersion] = useState(0);
  const [isHoverToolbarForcedVisible, setIsHoverToolbarForcedVisible] =
    useState(false);
  const [error, setError] = useState<string | null>(null);
  const imageFrameSize = useMemo(() => {
    if (!image) return null;

    return isThumbnailMode
      ? getPinnedThumbnailDisplaySize(image)
      : getPinnedDisplaySize(image, zoom);
  }, [image, isThumbnailMode, zoom]);

  const hideCurrentPinnedImage = useCallback(async () => {
    try {
      await closePinnedImage(imageId);
    } catch (err) {
      console.error('Failed to hide pinned image:', err);
    }
  }, [imageId]);

  const destroyCurrentPinnedImage = useCallback(async () => {
    try {
      await destroyPinnedImage(imageId, webviewWindow);
    } catch (err) {
      console.error('Failed to destroy pinned image:', err);
    }
  }, [imageId]);

  const destroyCurrentPinnedImageGroup = useCallback(async () => {
    try {
      await destroyPinnedImageGroup(imageId);
      setContextMenuPosition(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }, [imageId]);

  const hideCurrentPinnedImageGroup = useCallback(async () => {
    try {
      await hidePinnedImageGroup(imageId);
      setContextMenuPosition(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }, [imageId]);

  useEffect(() => {
    let disposed = false;

    getPinnedImage(imageId)
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
    sampleCanvasRef.current = null;
    setCursorColor(null);
    setSampleCanvasVersion((version) => version + 1);

    if (!image) return;

    let disposed = false;
    const imageElement = new Image();
    imageElement.onload = () => {
      if (disposed) return;

      const canvas = document.createElement('canvas');
      canvas.width = imageElement.naturalWidth;
      canvas.height = imageElement.naturalHeight;
      canvas.getContext('2d')?.drawImage(imageElement, 0, 0);
      sampleCanvasRef.current = canvas;
      setSampleCanvasVersion((version) => version + 1);
    };
    imageElement.src = `data:image/png;base64,${image.image_base64}`;

    return () => {
      disposed = true;
    };
  }, [image]);

  useEffect(() => {
    if (!imagePointerPoint || !imageFrameSize) {
      setCursorColor(null);
      return;
    }

    const canvas = sampleCanvasRef.current;
    if (!canvas) {
      setCursorColor(null);
      return;
    }

    setCursorColor(
      sampleCanvasColor(canvas, imagePointerPoint, imageFrameSize),
    );
  }, [imageFrameSize, imagePointerPoint, sampleCanvasVersion]);

  const copyCurrentColor = useCallback(async () => {
    if (!cursorColor) return;

    try {
      await writeClipboardText(
        colorSampleToClipboardText(cursorColor, colorSampleFormat),
      );
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }, [colorSampleFormat, cursorColor]);

  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key !== 'Escape') return;

      const toolbarAction = getPinnedKeyboardToolbarAction(
        event,
        isHoverToolbarForcedVisible,
      );
      if (toolbarAction === 'hide') {
        event.preventDefault();
        setIsHoverToolbarForcedVisible(false);
        return;
      }

      event.preventDefault();
      if (isDestroyPinnedImageShortcut(event)) {
        void destroyCurrentPinnedImage();
        return;
      }

      void hideCurrentPinnedImage();
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [
    destroyCurrentPinnedImage,
    hideCurrentPinnedImage,
    isHoverToolbarForcedVisible,
  ]);

  const resizePinnedWindow = useCallback(
    async (
      nextZoom: number,
      nextTransform = transform,
      nextImage = image,
      nextThumbnailMode = isThumbnailMode,
    ) => {
      if (!nextImage) return;

      const size = getPinnedDisplaySizeForTransform(
        nextImage,
        nextZoom,
        nextTransform,
        nextThumbnailMode,
      );
      await appWindow.setSize(createLogicalSize(size.width, size.height));
    },
    [image, isThumbnailMode, transform],
  );

  const resetPinnedSize = useCallback(() => {
    setZoom(1);
    setIsThumbnailMode(false);
    setContextMenuPosition(null);
    void resizePinnedWindow(1, transform, image, false);
  }, [image, resizePinnedWindow, transform]);

  const resetPinnedSizeAndOpacity = useCallback(() => {
    setZoom(1);
    setOpacity(1);
    setIsThumbnailMode(false);
    setContextMenuPosition(null);
    void resizePinnedWindow(1, transform, image, false);
  }, [image, resizePinnedWindow, transform]);

  const setPinnedOpacityPreset = useCallback((nextOpacity: number) => {
    setOpacity(getPinnedOpacityPreset(nextOpacity));
    setContextMenuPosition(null);
  }, []);

  const copyCurrentPinnedImage = useCallback(async () => {
    try {
      await copyPinnedImage(imageId);
      setContextMenuPosition(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }, [imageId]);

  const copyPinnedSourceText = useCallback(async () => {
    try {
      await copyPinnedText(
        writeClipboardText,
        image?.source_text,
      );
      setContextMenuPosition(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }, [image?.source_text]);

  const savePinnedImageAs = useCallback(async () => {
    try {
      await savePinnedImage(imageId);
      setContextMenuPosition(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }, [imageId]);

  const quickSavePinnedImageToDirectory = useCallback(async () => {
    try {
      await quickSavePinnedImage(imageId, screenshotSavePath);
      setContextMenuPosition(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }, [imageId, screenshotSavePath]);

  const openPreferencesWindow = useCallback(async () => {
    try {
      await openPinnedPreferences(() => getWebviewWindowByLabel('settings'));
      setContextMenuPosition(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }, []);

  const replacePinnedFromClipboard = useCallback(async () => {
    try {
      const nextImage = await replacePinnedImageFromClipboard<PinnedImageView>(
        imageId,
      );
      const nextTransform = createDefaultPinnedTransform();

      setImage(nextImage);
      setZoom(1);
      setIsThumbnailMode(false);
      setTransform(nextTransform);
      setContextMenuPosition(null);
      await resizePinnedWindow(1, nextTransform, nextImage, false);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }, [imageId, resizePinnedWindow]);

  const movePinnedToAnotherGroup = useCallback(async () => {
    try {
      await movePinnedImageToNextGroup(imageId);
      setContextMenuPosition(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }, [imageId]);

  const adjustPinnedZoom = useCallback(
    (wheelDirection: number) => {
      setIsThumbnailMode(false);
      setContextMenuPosition(null);
      setZoom((currentZoom) => {
        const nextZoom = getPinnedZoomFromWheel(currentZoom, wheelDirection);
        void resizePinnedWindow(nextZoom, transform, image, false);
        return nextZoom;
      });
    },
    [image, resizePinnedWindow, transform],
  );

  const togglePinnedThumbnailMode = useCallback(() => {
    setContextMenuPosition(null);
    setIsThumbnailMode((currentThumbnailMode) => {
      const nextThumbnailMode = !currentThumbnailMode;
      void resizePinnedWindow(zoom, transform, image, nextThumbnailMode);
      return nextThumbnailMode;
    });
  }, [image, resizePinnedWindow, transform, zoom]);

  const movePinnedWindowByKeyboard = useCallback(
    async (delta: { x: number; y: number }) => {
      try {
        setContextMenuPosition(null);
        const position = await appWindow.outerPosition();
        await appWindow.setPosition(
          createPhysicalPosition(position.x + delta.x, position.y + delta.y),
        );
      } catch (err) {
        setError(err instanceof Error ? err.message : String(err));
      }
    },
    [],
  );

  const startPinnedWindowDrag = useCallback(async () => {
    try {
      await appWindow.startDragging();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }, []);

  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      if (isPinnedMagnifierShortcut(event)) {
        event.preventDefault();
        setIsMagnifierRequested(true);
        return;
      }

      if (
        isMagnifierRequested &&
        cursorColor &&
        isColorSampleCopyShortcut(event)
      ) {
        event.preventDefault();
        void copyCurrentColor();
        return;
      }

      if (
        isMagnifierRequested &&
        cursorColor &&
        isColorSampleFormatToggleShortcut(event)
      ) {
        event.preventDefault();
        setColorSampleFormat((format) => (format === 'hex' ? 'rgb' : 'hex'));
        return;
      }

      if (isCopyPinnedTextShortcut(event, image?.source_text)) {
        event.preventDefault();
        void copyPinnedSourceText();
        return;
      }

      if (isCopyPinnedImageShortcut(event)) {
        event.preventDefault();
        void copyCurrentPinnedImage();
        return;
      }

      if (isSavePinnedImageShortcut(event)) {
        event.preventDefault();
        void savePinnedImageAs();
        return;
      }

      if (isQuickSavePinnedImageShortcut(event)) {
        event.preventDefault();
        void quickSavePinnedImageToDirectory();
        return;
      }

      if (isOpenPinnedPreferencesShortcut(event)) {
        event.preventDefault();
        void openPreferencesWindow();
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

      const toolbarAction = getPinnedKeyboardToolbarAction(
        event,
        isHoverToolbarForcedVisible,
      );
      if (toolbarAction === 'toggle') {
        event.preventDefault();
        setContextMenuPosition(null);
        setIsHoverToolbarForcedVisible((currentVisible) => !currentVisible);
        return;
      }

      const moveDelta = getPinnedKeyboardMoveDelta(event);
      if (moveDelta) {
        event.preventDefault();
        void movePinnedWindowByKeyboard(moveDelta);
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
          void resizePinnedWindow(zoom, nextTransform, image, isThumbnailMode);
          return nextTransform;
        });
        return;
      }

      const visualFilterAction = getPinnedKeyboardVisualFilterAction(event);
      if (visualFilterAction) {
        event.preventDefault();
        setContextMenuPosition(null);
        setVisualFilter((currentFilter) =>
          nextPinnedVisualFilter(currentFilter, visualFilterAction),
        );
      }
    };

    const handleKeyUp = (event: KeyboardEvent) => {
      if (event.key === 'Alt') {
        setIsMagnifierRequested(false);
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    window.addEventListener('keyup', handleKeyUp);
    return () => {
      window.removeEventListener('keydown', handleKeyDown);
      window.removeEventListener('keyup', handleKeyUp);
    };
  }, [
    adjustPinnedZoom,
    copyCurrentColor,
    copyCurrentPinnedImage,
    copyPinnedSourceText,
    cursorColor,
    hideCurrentPinnedImage,
    image?.source_text,
    isMagnifierRequested,
    isHoverToolbarForcedVisible,
    isThumbnailMode,
    movePinnedWindowByKeyboard,
    openPreferencesWindow,
    quickSavePinnedImageToDirectory,
    replacePinnedFromClipboard,
    resetPinnedSize,
    resizePinnedWindow,
    savePinnedImageAs,
    startPinnedWindowDrag,
    image,
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

  const handlePinnedImagePointerMove = (
    event: React.PointerEvent<HTMLDivElement>,
  ) => {
    const frame = imageFrameRef.current?.getBoundingClientRect();
    if (!frame) return;

    setViewportPointerPoint({ x: event.clientX, y: event.clientY });
    setImagePointerPoint(
      getPinnedImagePointFromPointer(
        { x: event.clientX, y: event.clientY },
        frame,
      ),
    );
  };

  const handlePinnedImagePointerLeave = () => {
    setViewportPointerPoint(null);
    setImagePointerPoint(null);
  };

  if (error) {
    return (
      <div className="flex h-screen w-screen items-center justify-center bg-neutral-950 p-3 text-xs text-red-100">
        {error}
      </div>
    );
  }

  const isMagnifierShown = Boolean(
    isMagnifierRequested &&
      image &&
      imageFrameSize &&
      imagePointerPoint &&
      viewportPointerPoint &&
      cursorColor,
  );
  const magnifierPosition =
    isMagnifierShown && viewportPointerPoint
      ? getMagnifierPosition(
          viewportPointerPoint,
          {
            x: 0,
            y: 0,
            width: window.innerWidth,
            height: window.innerHeight,
          },
          PIN_MAGNIFIER_SIZE,
          PIN_MAGNIFIER_GAP,
        )
      : null;
  const magnifierImageStyle =
    isMagnifierShown && image && imageFrameSize && imagePointerPoint
      ? getMagnifierImageStyle(
          image.image_base64,
          imagePointerPoint,
          imageFrameSize,
          PIN_MAGNIFIER_SIZE,
          PIN_MAGNIFIER_ZOOM,
        )
      : null;
  const colorText = cursorColor
    ? colorSampleToClipboardText(cursorColor, colorSampleFormat)
    : '';
  const runHoverToolbarAction = (actionId: typeof PIN_HOVER_TOOLBAR_ACTIONS[number]['id']) => {
    setIsHoverToolbarForcedVisible(false);

    if (actionId === 'copy') {
      void copyCurrentPinnedImage();
      return;
    }

    if (actionId === 'save') {
      void savePinnedImageAs();
      return;
    }

    void hideCurrentPinnedImage();
  };
  const hoverToolbarVisibilityClassName = isHoverToolbarForcedVisible
    ? 'opacity-100'
    : 'opacity-0 focus-within:opacity-100 group-hover:opacity-100';

  return (
    <div
      className="group relative h-screen w-screen overflow-hidden bg-transparent"
      onWheel={handleWheel}
      onContextMenu={handleContextMenu}
      onPointerDown={() => setContextMenuPosition(null)}
    >
      {image && imageFrameSize && (
        <div
          ref={imageFrameRef}
          className="absolute left-1/2 top-1/2"
          style={{
            width: `${imageFrameSize.width}px`,
            height: `${imageFrameSize.height}px`,
            transform: 'translate(-50%, -50%)',
          }}
          onPointerMove={handlePinnedImagePointerMove}
          onPointerLeave={handlePinnedImagePointerLeave}
        >
          <img
            src={`data:image/png;base64,${image.image_base64}`}
            className="h-full w-full object-fill"
            style={{
              opacity,
              filter: getPinnedVisualFilterStyle(visualFilter),
              transform: getPinnedTransformStyle(transform),
            }}
            draggable={false}
            onPointerDown={(event) => {
              if (isTogglePinnedThumbnailModeDoubleClick(event)) {
                event.preventDefault();
                togglePinnedThumbnailMode();
                return;
              }

              if (isClosePinnedImageDoubleClick(event)) {
                event.preventDefault();
                void hideCurrentPinnedImage();
                return;
              }

              if (isResetPinnedImagePointer(event)) {
                event.preventDefault();
                resetPinnedSizeAndOpacity();
                return;
              }

              if (event.button !== 0) return;
              void startPinnedWindowDrag();
            }}
          />
        </div>
      )}
      {isMagnifierShown &&
        magnifierPosition &&
        magnifierImageStyle &&
        cursorColor && (
          <div
            className="pointer-events-none absolute overflow-hidden rounded border border-white/70 bg-neutral-950 text-[10px] text-white shadow-2xl ring-1 ring-black/50"
            style={{
              left: `${magnifierPosition.x}px`,
              top: `${magnifierPosition.y}px`,
              width: `${PIN_MAGNIFIER_SIZE.width}px`,
              height: `${PIN_MAGNIFIER_SIZE.height}px`,
            }}
          >
            <div className="relative h-full w-full">
              <div
                className="absolute inset-0"
                style={{
                  ...magnifierImageStyle,
                  imageRendering: 'pixelated',
                }}
              />
              <div className="absolute left-1/2 top-0 h-full w-px -translate-x-1/2 bg-white/65" />
              <div className="absolute left-0 top-1/2 h-px w-full -translate-y-1/2 bg-white/65" />
              <div className="absolute bottom-1 left-1 flex items-center gap-1 rounded bg-black/70 px-1.5 py-0.5">
                <span
                  className="h-2.5 w-2.5 rounded-sm border border-white/50"
                  style={{ backgroundColor: cursorColor.hex }}
                />
                <span>{colorText}</span>
              </div>
            </div>
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
            Close
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
            onClick={copyCurrentPinnedImage}
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
      <div
        className={`absolute right-1 top-1 flex gap-1 rounded bg-black/70 p-1 text-xs text-white shadow transition-opacity ${hoverToolbarVisibilityClassName}`}
        onPointerDown={(event) => event.stopPropagation()}
      >
        {PIN_HOVER_TOOLBAR_ACTIONS.map((action) => (
          <button
            key={action.id}
            type="button"
            className="h-6 rounded px-2 leading-6 hover:bg-white/15 focus:bg-white/15"
            aria-label={action.ariaLabel}
            title={action.title}
            onClick={() => runHoverToolbarAction(action.id)}
          >
            {action.label}
          </button>
        ))}
      </div>
    </div>
  );
}

export function readPinnedImageLaunch(search: string) {
  return readPinnedImageId(search);
}
