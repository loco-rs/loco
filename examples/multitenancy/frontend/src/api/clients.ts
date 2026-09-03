import {
  useMutation,
  useQuery,
  useQueryClient,
  type UseMutationResult,
  type UseQueryResult,
} from "@tanstack/react-query";
import type { SelectedWorkspace } from "../auth/session";
import type { ClientDto } from "../bindings/ClientDto";
import type { CreateClient } from "../bindings/CreateClient";
import type { UpdateClient } from "../bindings/UpdateClient";
import { ApiClientError, get, post, put } from "./client";

export const clientKeys = {
  list: (workspace: SelectedWorkspace) => ["clients", workspace.tenantId] as const,
  detail: (workspace: SelectedWorkspace, id: number) => ["clients", workspace.tenantId, id] as const,
};

const path = (workspace: SelectedWorkspace) => `/api/tenants/${workspace.tenantId}/clients`;

export function useClients(workspace: SelectedWorkspace): UseQueryResult<ClientDto[], ApiClientError> {
  return useQuery({ queryKey: clientKeys.list(workspace), queryFn: () => get<ClientDto[]>(path(workspace)) });
}

export function useClient(workspace: SelectedWorkspace, id: number): UseQueryResult<ClientDto, ApiClientError> {
  return useQuery({ queryKey: clientKeys.detail(workspace, id), queryFn: () => get<ClientDto>(`${path(workspace)}/${id}`), enabled: Number.isInteger(id) });
}

export function useCreateClient(workspace: SelectedWorkspace): UseMutationResult<ClientDto, ApiClientError, CreateClient> {
  const queryClient = useQueryClient();
  return useMutation({ mutationFn: (client) => post<ClientDto>(path(workspace), client), onSuccess: () => void queryClient.invalidateQueries({ queryKey: clientKeys.list(workspace) }) });
}

export function useUpdateClient(workspace: SelectedWorkspace, id: number): UseMutationResult<ClientDto, ApiClientError, UpdateClient> {
  const queryClient = useQueryClient();
  return useMutation({ mutationFn: (client) => put<ClientDto>(`${path(workspace)}/${id}`, client), onSuccess: (client) => { queryClient.setQueryData(clientKeys.detail(workspace, id), client); void queryClient.invalidateQueries({ queryKey: clientKeys.list(workspace) }); } });
}
