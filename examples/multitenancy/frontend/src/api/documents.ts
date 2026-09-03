import {
  useMutation,
  useQuery,
  useQueryClient,
  type UseMutationResult,
  type UseQueryResult,
} from "@tanstack/react-query";
import type { AccessContext } from "../auth/access";
import type { CreateDocument } from "../bindings/CreateDocument";
import type { DocumentDto } from "../bindings/DocumentDto";
import { ApiClientError, get, post } from "./client";

export function documentsPath(access: AccessContext): string {
  return `/api/tenants/${access.tenantId}/applications/${access.applicationId}/documents`;
}

export const documentKeys = {
  list: (access: AccessContext) =>
    ["documents", access.tenantId, access.applicationId] as const,
};

export function useDocuments(
  access: AccessContext,
): UseQueryResult<DocumentDto[], ApiClientError> {
  return useQuery({
    queryKey: documentKeys.list(access),
    queryFn: () => get<DocumentDto[]>(documentsPath(access)),
  });
}

export function useCreateDocument(
  access: AccessContext,
): UseMutationResult<DocumentDto, ApiClientError, CreateDocument> {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (document) =>
      post<DocumentDto>(documentsPath(access), document),
    onSuccess: () => {
      void queryClient.invalidateQueries({
        queryKey: documentKeys.list(access),
      });
    },
  });
}
