export function getPinnedWindowRequiredPermissions() {
  return [
    'core:window:allow-close',
    'core:window:allow-set-position',
    'core:window:allow-set-size',
    'core:window:allow-start-dragging',
  ] as const;
}
