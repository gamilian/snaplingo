import { invoke } from '@tauri-apps/api/core';
import type {
  RequiredPermissionsPort,
  RequiredPermissionsStatus,
} from '../../application/permissions/runtime';

export const requiredPermissions: RequiredPermissionsPort = {
  status: () =>
    invoke<RequiredPermissionsStatus>('get_required_permissions_status'),
  request: () =>
    invoke<RequiredPermissionsStatus>('request_required_permissions'),
};
