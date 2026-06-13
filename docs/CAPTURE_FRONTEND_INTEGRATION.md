# Capture Frontend Integration Guide

This guide shows frontend developers how to integrate with SnapLingo's capture service backend.

## Backend Commands

Three Tauri commands are available for screenshot capture:

### 1. `capture_full_screen()`

Captures the entire screen and returns base64-encoded PNG data.

**Returns:** `Promise<string>` - Base64-encoded PNG image

### 2. `capture_region(x, y, width, height)`

Captures a specific screen region and returns base64-encoded PNG data.

**Parameters:**
- `x: number` - X coordinate (pixels from left)
- `y: number` - Y coordinate (pixels from top)
- `width: number` - Width in pixels
- `height: number` - Height in pixels

**Returns:** `Promise<string>` - Base64-encoded PNG image

### 3. `save_screenshot(data, path)`

Saves base64-encoded PNG data to disk.

**Parameters:**
- `data: string` - Base64-encoded PNG data
- `path: string` - File system path where to save

**Returns:** `Promise<void>`

## Usage Examples

### Capture Full Screen

```typescript
import { invoke } from '@tauri-apps/api/core';

async function captureScreen() {
  try {
    const base64Image = await invoke<string>('capture_full_screen');
    console.log('Captured image:', base64Image);
    return base64Image;
  } catch (error) {
    console.error('Failed to capture screen:', error);
    throw error;
  }
}
```

### Capture Region

```typescript
async function captureRegion(x: number, y: number, width: number, height: number) {
  try {
    const base64Image = await invoke<string>('capture_region', {
      x,
      y,
      width,
      height
    });
    return base64Image;
  } catch (error) {
    console.error('Failed to capture region:', error);
    throw error;
  }
}

// Example: Capture 400x300 region starting at (100, 100)
const imageData = await captureRegion(100, 100, 400, 300);
```

### Display Captured Image

```typescript
function displayScreenshot(base64Image: string) {
  // Create data URI for display
  const dataUri = `data:image/png;base64,${base64Image}`;
  
  // Update img element
  const imgElement = document.getElementById('screenshot') as HTMLImageElement;
  imgElement.src = dataUri;
  
  // Or use in React/Vue/etc
  // setImageSrc(dataUri);
}

// Complete flow
async function captureAndDisplay() {
  const imageData = await captureScreen();
  displayScreenshot(imageData);
}
```

### Save Screenshot

```typescript
import { save } from '@tauri-apps/plugin-dialog';

async function saveScreenshot(base64Image: string) {
  try {
    // Show save dialog
    const filePath = await save({
      defaultPath: 'screenshot.png',
      filters: [{
        name: 'PNG Image',
        extensions: ['png']
      }]
    });
    
    if (filePath) {
      await invoke('save_screenshot', {
        data: base64Image,
        path: filePath
      });
      console.log('Screenshot saved to:', filePath);
    }
  } catch (error) {
    console.error('Failed to save screenshot:', error);
    throw error;
  }
}
```

### Convert Base64 to Blob (for upload/processing)

```typescript
function base64ToBlob(base64: string): Blob {
  const byteString = atob(base64);
  const arrayBuffer = new ArrayBuffer(byteString.length);
  const uint8Array = new Uint8Array(arrayBuffer);
  
  for (let i = 0; i < byteString.length; i++) {
    uint8Array[i] = byteString.charCodeAt(i);
  }
  
  return new Blob([arrayBuffer], { type: 'image/png' });
}

// Usage
const imageBlob = base64ToBlob(base64Image);
const formData = new FormData();
formData.append('image', imageBlob, 'screenshot.png');
```

## Implementation Checklist

Use this checklist when integrating capture functionality:

### UI Components
- [ ] Add hotkey display/configuration UI
- [ ] Create screenshot preview component
- [ ] Implement region selection overlay (if needed)
- [ ] Add save/cancel buttons
- [ ] Show loading state during capture

### Hotkey Integration
- [ ] Register global hotkeys via backend (see `hotkey` module)
- [ ] Handle hotkey events from backend
- [ ] Trigger `capture_full_screen()` on hotkey
- [ ] Show/hide capture window appropriately

### Screenshot Display
- [ ] Decode base64 to display captured image
- [ ] Handle large screenshots (optimize rendering)
- [ ] Add zoom/pan controls (optional)
- [ ] Implement crop/region selection (if needed)

### Save Functionality
- [ ] Integrate Tauri dialog plugin for save dialog
- [ ] Call `save_screenshot()` with user-selected path
- [ ] Show success/error notifications
- [ ] Handle save errors gracefully

### Error Handling
- [ ] Handle capture failures (permissions, etc.)
- [ ] Display user-friendly error messages
- [ ] Handle base64 decode errors
- [ ] Handle file system errors on save

## Platform Notes

### macOS
- Requires Screen Recording permission (granted via System Preferences)
- Backend handles permission requests automatically
- Full screen and region capture both supported

### Windows/Linux
- Backend stubs currently return placeholder data
- Full implementation coming in future phases

## Next Steps

1. Review existing hotkey implementation in `src-tauri/src/hotkeys/`
2. Implement UI components for screenshot workflow
3. Test capture on macOS
4. Add OCR integration after capture (see `recognize_image` command)
