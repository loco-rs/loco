import { useMutation, useQuery, useQueryClient, type UseQueryResult } from "@tanstack/react-query";
import type { CreateWorkspace } from "../bindings/CreateWorkspace";
import type { LoginResponse } from "../bindings/LoginResponse";
import type { RegisterTenant } from "../bindings/RegisterTenant";
import type { RegisterTenantResponse } from "../bindings/RegisterTenantResponse";
import type { Workspace } from "../bindings/Workspace";
import { ApiClientError, get, post } from "./client";

export interface LoginRequest {
  email: string;
  password: string;
}

const workspacesQueryKey = ["auth", "workspaces"] as const;

export function login(params: LoginRequest): Promise<LoginResponse> {
  return post<LoginResponse>("/api/auth/login", params);
}

export function registerTenant(
  params: RegisterTenant,
): Promise<RegisterTenantResponse> {
  return post<RegisterTenantResponse>("/api/auth/register-tenant", params);
}

export function useWorkspaces(): UseQueryResult<Workspace[], ApiClientError> {
  return useQuery({
    queryKey: workspacesQueryKey,
    queryFn: () => get<Workspace[]>("/api/auth/workspaces"),
  });
}

export function useCreateWorkspace() {
  const queryClient = useQueryClient();

  return useMutation<Workspace, ApiClientError, CreateWorkspace>({
    mutationFn: (params) => post<Workspace>("/api/auth/workspaces", params),
    onSuccess: (workspace) => {
      queryClient.setQueryData<Workspace[]>(workspacesQueryKey, (current) =>
        current ? [...current, workspace] : [workspace],
      );
    },
  });
}
