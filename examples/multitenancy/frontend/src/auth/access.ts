const ACCESS_KEY = "loco_multitenancy_access";

export interface AccessContext {
  apiKey: string;
  tenantId: number;
  applicationId: number;
}

export function isAccessContext(value: unknown): value is AccessContext {
  if (typeof value !== "object" || value === null) {
    return false;
  }

  const access = value as Partial<AccessContext>;
  return (
    typeof access.apiKey === "string" &&
    access.apiKey.length > 0 &&
    typeof access.tenantId === "number" &&
    Number.isInteger(access.tenantId) &&
    access.tenantId > 0 &&
    typeof access.applicationId === "number" &&
    Number.isInteger(access.applicationId) &&
    access.applicationId > 0
  );
}

export function loadAccess(): AccessContext | null {
  const stored = window.localStorage.getItem(ACCESS_KEY);
  if (stored === null) {
    return null;
  }

  try {
    const value: unknown = JSON.parse(stored);
    return isAccessContext(value) ? value : null;
  } catch {
    return null;
  }
}

export function hasAccess(): boolean {
  return loadAccess() !== null;
}

export function saveAccess(access: AccessContext): void {
  window.localStorage.setItem(ACCESS_KEY, JSON.stringify(access));
}

export function clearAccess(): void {
  window.localStorage.removeItem(ACCESS_KEY);
}
