import type { ApiClientError } from "../api/client";
import type { Workspace } from "../bindings/Workspace";
import type { SelectedWorkspace } from "./session";

export const CREATE_WORKSPACE_VALUE = "create-workspace";

export function flattenWorkspaces(
  workspaces: Workspace[] | undefined,
): SelectedWorkspace[] {
  return (workspaces ?? []).flatMap((workspace) =>
    workspace.applications.map((application) => ({
      tenantId: workspace.tenant_id,
      tenantName: workspace.tenant_name,
      applicationId: application.id,
      applicationName: application.name,
    })),
  );
}

export function resolveWorkspace(
  options: SelectedWorkspace[],
  saved: SelectedWorkspace | null,
): SelectedWorkspace | undefined {
  return (
    options.find(
      (option) =>
        option.tenantId === saved?.tenantId &&
        option.applicationId === saved.applicationId,
    ) ?? options[0]
  );
}

export function workspaceSelection(
  workspace: Workspace,
): SelectedWorkspace | undefined {
  const application = workspace.applications[0];
  return application
    ? {
        tenantId: workspace.tenant_id,
        tenantName: workspace.tenant_name,
        applicationId: application.id,
        applicationName: application.name,
      }
    : undefined;
}

export interface WorkspaceOutletContext {
  selected: SelectedWorkspace | undefined;
  options: SelectedWorkspace[];
  isLoading: boolean;
  error: ApiClientError | null;
  openWorkspaceCreator: () => void;
  selectApplication: (applicationName: string) => void;
}
