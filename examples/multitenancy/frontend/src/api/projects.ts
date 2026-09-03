import {
  useMutation,
  useQuery,
  useQueryClient,
  type UseMutationResult,
  type UseQueryResult,
} from "@tanstack/react-query";
import type { SelectedWorkspace } from "../auth/session";
import type { CreateProject } from "../bindings/CreateProject";
import type { ProjectDto } from "../bindings/ProjectDto";
import type { UpdateProject } from "../bindings/UpdateProject";
import { ApiClientError, get, post, put } from "./client";

export const projectKeys = {
  list: (workspace: SelectedWorkspace) => ["projects", workspace.tenantId] as const,
  detail: (workspace: SelectedWorkspace, id: number) => ["projects", workspace.tenantId, id] as const,
};

const path = (workspace: SelectedWorkspace) => `/api/tenants/${workspace.tenantId}/projects`;

export function useProjects(workspace: SelectedWorkspace): UseQueryResult<ProjectDto[], ApiClientError> {
  return useQuery({ queryKey: projectKeys.list(workspace), queryFn: () => get<ProjectDto[]>(path(workspace)) });
}

export function useProject(workspace: SelectedWorkspace, id: number): UseQueryResult<ProjectDto, ApiClientError> {
  return useQuery({ queryKey: projectKeys.detail(workspace, id), queryFn: () => get<ProjectDto>(`${path(workspace)}/${id}`), enabled: Number.isInteger(id) });
}

export function useCreateProject(workspace: SelectedWorkspace): UseMutationResult<ProjectDto, ApiClientError, CreateProject> {
  const queryClient = useQueryClient();
  return useMutation({ mutationFn: (project) => post<ProjectDto>(path(workspace), project), onSuccess: () => void queryClient.invalidateQueries({ queryKey: projectKeys.list(workspace) }) });
}

export function useUpdateProject(workspace: SelectedWorkspace, id: number): UseMutationResult<ProjectDto, ApiClientError, UpdateProject> {
  const queryClient = useQueryClient();
  return useMutation({ mutationFn: (project) => put<ProjectDto>(`${path(workspace)}/${id}`, project), onSuccess: (project) => { queryClient.setQueryData(projectKeys.detail(workspace, id), project); void queryClient.invalidateQueries({ queryKey: projectKeys.list(workspace) }); } });
}
