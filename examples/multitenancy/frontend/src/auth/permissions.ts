import type { PermissionAccess } from "../bindings/PermissionAccess";

export function hasPermission(
  permissions: PermissionAccess[] | undefined,
  key: string,
): boolean {
  return permissions?.some((permission) => permission.key === key) ?? false;
}
