export interface RequiredPermissionsStatus {
  screenRecording: boolean;
  accessibility: boolean;
}

export interface RequiredPermissionsPort {
  status(): Promise<RequiredPermissionsStatus>;
  request(): Promise<RequiredPermissionsStatus>;
}

export interface RequiredPermissionsRuntime extends RequiredPermissionsPort {}

export function createRequiredPermissionsRuntime(
  port: RequiredPermissionsPort,
): RequiredPermissionsRuntime {
  return {
    status: () => port.status(),
    request: () => port.request(),
  };
}

export function areRequiredPermissionsGranted(
  status: RequiredPermissionsStatus,
) {
  return status.screenRecording && status.accessibility;
}
