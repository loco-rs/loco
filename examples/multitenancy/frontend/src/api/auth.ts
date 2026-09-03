import { useQuery, type UseQueryResult } from "@tanstack/react-query";
import type { LoginResponse } from "../bindings/LoginResponse";
import type { RegisterTenant } from "../bindings/RegisterTenant";
import type { RegisterTenantResponse } from "../bindings/RegisterTenantResponse";
import type { Workspace } from "../bindings/Workspace";
import { ApiClientError, get, post } from "./client";

export interface LoginRequest {
  email: string;
  password: string;
}

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
    queryKey: ["auth", "workspaces"],
    queryFn: () => get<Workspace[]>("/api/auth/workspaces"),
  });
}
