import type { ApiClientError } from "../api/client";
import type { Workspace } from "../bindings/Workspace";
import type { SelectedWorkspace } from "./session";

export const CREATE_WORKSPACE_VALUE = "create-workspace";

export function flattenWorkspaces(
  workspaces: Workspace[] | undefined,
): SelectedWorkspace[] {
  return (workspaces ?? []).map((workspace) => ({
    tenantId: workspace.tenant_id,
    tenantName: workspace.tenant_name,
  }));
}

export function resolveWorkspace(
  options: SelectedWorkspace[],
  saved: SelectedWorkspace | null,
): SelectedWorkspace | undefined {
  return (
    options.find(
      (option) =>
        option.tenantId === saved?.tenantId,
    ) ?? options[0]
  );
}

export function workspaceSelection(workspace: Workspace): SelectedWorkspace {
  return {
    tenantId: workspace.tenant_id,
    tenantName: workspace.tenant_name,
  };
}

export interface WorkspaceOutletContext {
  selected: SelectedWorkspace | undefined;
  options: SelectedWorkspace[];
  isLoading: boolean;
  error: ApiClientError | null;
  openWorkspaceCreator: () => void;
}
