import { useQuery, type UseQueryResult } from "@tanstack/react-query";
import type { DashboardDto } from "../bindings/DashboardDto";
import { ApiClientError, get } from "./client";

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
