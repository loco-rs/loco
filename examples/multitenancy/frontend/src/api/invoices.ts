import {
  useMutation,
  useQuery,
  useQueryClient,
  type UseMutationResult,
  type UseQueryResult,
} from "@tanstack/react-query";
import type { SelectedWorkspace } from "../auth/session";
import type { CreateInvoice } from "../bindings/CreateInvoice";
import type { InvoiceDto } from "../bindings/InvoiceDto";
import { dashboardKeys } from "./dashboard";
import { ApiClientError, get, post } from "./client";

export function invoicesPath(workspace: SelectedWorkspace): string {
  return `/api/tenants/${workspace.tenantId}/applications/${workspace.applicationId}/invoices`;
}

export const invoiceKeys = {
  list: (workspace: SelectedWorkspace) =>
    ["invoices", workspace.tenantId, workspace.applicationId] as const,
};

export function useInvoices(
  workspace: SelectedWorkspace,
): UseQueryResult<InvoiceDto[], ApiClientError> {
  return useQuery({
    queryKey: invoiceKeys.list(workspace),
    queryFn: () => get<InvoiceDto[]>(invoicesPath(workspace)),
  });
}

export function useCreateInvoice(
  workspace: SelectedWorkspace,
): UseMutationResult<InvoiceDto, ApiClientError, CreateInvoice> {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (invoice) => post<InvoiceDto>(invoicesPath(workspace), invoice),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: invoiceKeys.list(workspace) });
      void queryClient.invalidateQueries({
        queryKey: dashboardKeys.detail(workspace.tenantId),
      });
    },
  });
}
