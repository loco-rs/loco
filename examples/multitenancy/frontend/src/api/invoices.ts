import {
  useQuery,
  type UseQueryResult,
} from "@tanstack/react-query";
import type { SelectedWorkspace } from "../auth/session";
import type { InvoiceDto } from "../bindings/InvoiceDto";
import { ApiClientError, get } from "./client";

export function invoicesPath(workspace: SelectedWorkspace): string {
  return `/api/tenants/${workspace.tenantId}/invoices`;
}

export const invoiceKeys = {
  list: (workspace: SelectedWorkspace) =>
    ["invoices", workspace.tenantId] as const,
};

export function useInvoices(
  workspace: SelectedWorkspace,
): UseQueryResult<InvoiceDto[], ApiClientError> {
  return useQuery({
    queryKey: invoiceKeys.list(workspace),
    queryFn: () => get<InvoiceDto[]>(invoicesPath(workspace)),
  });
}
