import { useMutation, useQuery, useQueryClient, type UseQueryResult } from "@tanstack/react-query";
import type { CreateWorkspace } from "../bindings/CreateWorkspace";
import type { LoginResponse } from "../bindings/LoginResponse";
import type { RegisterAccount } from "../bindings/RegisterAccount";
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

export function registerAccount(params: RegisterAccount): Promise<LoginResponse> {
  return post<LoginResponse>("/api/auth/register-account", params);
}

export function useWorkspaces(
  enabled = true,
): UseQueryResult<Workspace[], ApiClientError> {
  return useQuery({
    queryKey: workspacesQueryKey,
    queryFn: () => get<Workspace[]>("/api/auth/workspaces"),
    enabled,
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
