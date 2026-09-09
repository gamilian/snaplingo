import { invoke } from '@tauri-apps/api/core';
import type {
  RequiredPermissionsPort,
  RequiredPermissionsStatus,
  RequiredPermissionsContext,
} from '../../application/permissions/runtime';

export const requiredPermissions: RequiredPermissionsPort = {
  status: () =>
    invoke<RequiredPermissionsStatus>('get_required_permissions_status'),
  context: () => invoke<RequiredPermissionsContext>('get_required_permissions_context'),
  request: (permission) =>
    invoke<RequiredPermissionsStatus>('request_required_permission', { permission }),
  reset: (permission) =>
    invoke<RequiredPermissionsStatus>('reset_required_permission', { permission }),
  restart: () => invoke('restart_after_permission_change'),
};
