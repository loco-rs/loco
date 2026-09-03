import {
  useMutation,
  useQuery,
  useQueryClient,
  type UseMutationResult,
  type UseQueryResult,
} from "@tanstack/react-query";
import type { DashboardDto } from "../bindings/DashboardDto";
import type { MemberRoleUpdate } from "../bindings/MemberRoleUpdate";
import type { UpdateMemberRole } from "../bindings/UpdateMemberRole";
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
