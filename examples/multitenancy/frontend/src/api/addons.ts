import {
  useMutation,
  useQueryClient,
  type UseMutationResult,
} from "@tanstack/react-query";
import type { SelectedWorkspace } from "../auth/session";
import type { InvoiceDto } from "../bindings/InvoiceDto";
import { ApiClientError, post } from "./client";
import { dashboardKeys } from "./dashboard";
import { invoiceKeys } from "./invoices";

export function usePurchaseAddon(
  workspace: SelectedWorkspace,
): UseMutationResult<InvoiceDto, ApiClientError, number> {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (addonId) =>
      post<InvoiceDto>(
        `/api/tenants/${workspace.tenantId}/addons/${addonId}/purchase`,
        {},
      ),
    onSuccess: () => {
      void queryClient.invalidateQueries({
        queryKey: dashboardKeys.detail(workspace.tenantId),
      });
      void queryClient.invalidateQueries({
        queryKey: invoiceKeys.list(workspace),
      });
    },
  });
}
