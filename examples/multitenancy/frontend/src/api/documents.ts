import {
  useMutation,
  useQuery,
  useQueryClient,
  type UseMutationResult,
  type UseQueryResult,
} from "@tanstack/react-query";
import type { SelectedWorkspace } from "../auth/session";
import type { CreateDocument } from "../bindings/CreateDocument";
import type { DocumentDto } from "../bindings/DocumentDto";
import { ApiClientError, get, post } from "./client";

export function documentsPath(workspace: SelectedWorkspace): string {
  return `/api/tenants/${workspace.tenantId}/applications/${workspace.applicationId}/documents`;
}

export const documentKeys = {
  list: (workspace: SelectedWorkspace) =>
    ["documents", workspace.tenantId, workspace.applicationId] as const,
};

export function useDocuments(
  workspace: SelectedWorkspace,
): UseQueryResult<DocumentDto[], ApiClientError> {
  return useQuery({
    queryKey: documentKeys.list(workspace),
    queryFn: () => get<DocumentDto[]>(documentsPath(workspace)),
  });
}

export function useCreateDocument(
  workspace: SelectedWorkspace,
): UseMutationResult<DocumentDto, ApiClientError, CreateDocument> {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (document) =>
      post<DocumentDto>(documentsPath(workspace), document),
    onSuccess: () => {
      void queryClient.invalidateQueries({
        queryKey: documentKeys.list(workspace),
      });
    },
  });
}
