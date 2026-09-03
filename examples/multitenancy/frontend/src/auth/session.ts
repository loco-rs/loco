const TOKEN_KEY = "loco_multitenancy_token";
const WORKSPACE_KEY = "loco_multitenancy_workspace";

export interface SelectedWorkspace {
  tenantId: number;
  tenantName: string;
}

export function getToken(): string | null {
  return window.localStorage.getItem(TOKEN_KEY);
}

export function setToken(token: string): void {
  window.localStorage.setItem(TOKEN_KEY, token);
}

export function isSelectedWorkspace(value: unknown): value is SelectedWorkspace {
  if (typeof value !== "object" || value === null) {
    return false;
  }

  const workspace = value as Partial<SelectedWorkspace>;
  return (
    typeof workspace.tenantId === "number" &&
    Number.isInteger(workspace.tenantId) &&
    workspace.tenantId > 0 &&
    typeof workspace.tenantName === "string"
  );
}

export function loadWorkspace(): SelectedWorkspace | null {
  const stored = window.localStorage.getItem(WORKSPACE_KEY);
  if (stored === null) {
    return null;
  }

  try {
    const value: unknown = JSON.parse(stored);
    return isSelectedWorkspace(value) ? value : null;
  } catch {
    return null;
  }
}

export function saveWorkspace(workspace: SelectedWorkspace): void {
  window.localStorage.setItem(WORKSPACE_KEY, JSON.stringify(workspace));
}

export function clearWorkspace(): void {
  window.localStorage.removeItem(WORKSPACE_KEY);
}

export function clearSession(): void {
  window.localStorage.removeItem(TOKEN_KEY);
  clearWorkspace();
}
