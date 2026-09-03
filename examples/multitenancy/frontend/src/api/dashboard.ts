import {
  useMutation,
  useQuery,
  useQueryClient,
  type UseMutationResult,
  type UseQueryResult,
} from "@tanstack/react-query";
import type { DashboardDto } from "../bindings/DashboardDto";
import type { MemberRoleUpdate } from "../bindings/MemberRoleUpdate";
import type { RolePermissionsUpdate } from "../bindings/RolePermissionsUpdate";
import type { UpdateMemberRole } from "../bindings/UpdateMemberRole";
import type { UpdateRolePermissions } from "../bindings/UpdateRolePermissions";
import { ApiClientError, get, post } from "./client";

export function dashboardPath(tenantId: number): string {
  return `/api/tenants/${tenantId}/dashboard`;
}

export const dashboardKeys = {
  detail: (tenantId: number) => ["dashboard", tenantId] as const,
};

export function useDashboard(
  tenantId: number,
): UseQueryResult<DashboardDto, ApiClientError> {
  return useQuery({
    queryKey: dashboardKeys.detail(tenantId),
    queryFn: () => get<DashboardDto>(dashboardPath(tenantId)),
  });
}

export function useUpdateMemberRole(
  tenantId: number,
  memberId: number,
): UseMutationResult<MemberRoleUpdate, ApiClientError, UpdateMemberRole> {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (params) =>
      post<MemberRoleUpdate>(
        `${dashboardPath(tenantId)}/members/${memberId}/role`,
        params,
      ),
    onSuccess: () => {
      void queryClient.invalidateQueries({
        queryKey: dashboardKeys.detail(tenantId),
      });
    },
  });
}

export function useUpdateRolePermissions(
  tenantId: number,
  roleId: number,
): UseMutationResult<
  RolePermissionsUpdate,
  ApiClientError,
  UpdateRolePermissions
> {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (params) =>
      post<RolePermissionsUpdate>(
        `${dashboardPath(tenantId)}/roles/${roleId}/permissions`,
        params,
      ),
    onSuccess: () => {
      void queryClient.invalidateQueries({
        queryKey: dashboardKeys.detail(tenantId),
      });
    },
  });
}
