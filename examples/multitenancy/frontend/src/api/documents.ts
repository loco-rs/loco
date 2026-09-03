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
import type { UpdateDocument } from "../bindings/UpdateDocument";
import { ApiClientError, get, post, put } from "./client";

export function documentsPath(workspace: SelectedWorkspace): string {
  return `/api/tenants/${workspace.tenantId}/documents`;
}

export const documentKeys = {
  list: (workspace: SelectedWorkspace) =>
    ["documents", workspace.tenantId] as const,
  detail: (workspace: SelectedWorkspace, id: number) =>
    ["documents", workspace.tenantId, id] as const,
};

export function useDocuments(
  workspace: SelectedWorkspace,
): UseQueryResult<DocumentDto[], ApiClientError> {
  return useQuery({
    queryKey: documentKeys.list(workspace),
    queryFn: () => get<DocumentDto[]>(documentsPath(workspace)),
  });
}

export function useDocument(
  workspace: SelectedWorkspace,
  id: number,
): UseQueryResult<DocumentDto, ApiClientError> {
  return useQuery({
    queryKey: documentKeys.detail(workspace, id),
    queryFn: () => get<DocumentDto>(`${documentsPath(workspace)}/${id}`),
    enabled: Number.isInteger(id),
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

export function useUpdateDocument(
  workspace: SelectedWorkspace,
  id: number,
): UseMutationResult<DocumentDto, ApiClientError, UpdateDocument> {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (document) =>
      put<DocumentDto>(`${documentsPath(workspace)}/${id}`, document),
    onSuccess: (document) => {
      queryClient.setQueryData(documentKeys.detail(workspace, id), document);
      void queryClient.invalidateQueries({ queryKey: documentKeys.list(workspace) });
    },
  });
}
