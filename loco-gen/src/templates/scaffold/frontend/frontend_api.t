to: frontend/src/api/{{ snake_plural }}.ts
skip_exists: true
message: "Frontend API hooks `{{ snake_plural }}.ts` were added successfully."
---
import {
  useMutation,
  useQuery,
  useQueryClient,
  type UseMutationResult,
  type UseQueryResult,
} from "@tanstack/react-query";
import type { Create{{ pascal_singular }} } from "../bindings/Create{{ pascal_singular }}";
import type { Page } from "../bindings/Page";
import type { {{ pascal_singular }}Dto } from "../bindings/{{ pascal_singular }}Dto";
import type { Update{{ pascal_singular }} } from "../bindings/Update{{ pascal_singular }}";
import { ApiClientError, del, get, post, put } from "./client";

export interface List{{ pascal_plural }}Params {
  page?: number;
  per_page?: number;
}

export const {{ camel_singular }}Keys = {
  all: ["{{ snake_plural }}"] as const,
  list: (params?: List{{ pascal_plural }}Params) =>
    ["{{ snake_plural }}", "list", params ?? {}] as const,
  detail: (id: number) => ["{{ snake_plural }}", "detail", id] as const,
};

function buildQueryString(params?: List{{ pascal_plural }}Params): string {
  if (!params) {
    return "";
  }
  const search = new URLSearchParams();
  if (params.page !== undefined) {
    search.set("page", String(params.page));
  }
  if (params.per_page !== undefined) {
    search.set("per_page", String(params.per_page));
  }
  const qs = search.toString();
  return qs ? `?${qs}` : "";
}

export function useList{{ pascal_plural }}(
  params?: List{{ pascal_plural }}Params,
): UseQueryResult<Page<{{ pascal_singular }}Dto>, ApiClientError> {
  return useQuery({
    queryKey: {{ camel_singular }}Keys.list(params),
    queryFn: () => get<Page<{{ pascal_singular }}Dto>>(`/api/{{ snake_plural }}${buildQueryString(params)}`),
  });
}

export function use{{ pascal_singular }}(id: number): UseQueryResult<{{ pascal_singular }}Dto, ApiClientError> {
  return useQuery({
    queryKey: {{ camel_singular }}Keys.detail(id),
    queryFn: () => get<{{ pascal_singular }}Dto>(`/api/{{ snake_plural }}/${id}`),
    enabled: Number.isFinite(id) && id > 0,
  });
}

export function useCreate{{ pascal_singular }}(): UseMutationResult<
  {{ pascal_singular }}Dto,
  ApiClientError,
  Create{{ pascal_singular }}
> {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (data: Create{{ pascal_singular }}) => post<{{ pascal_singular }}Dto>("/api/{{ snake_plural }}", data),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: {{ camel_singular }}Keys.all });
    },
  });
}

export interface Update{{ pascal_singular }}Variables {
  id: number;
  data: Update{{ pascal_singular }};
}

export function useUpdate{{ pascal_singular }}(): UseMutationResult<
  {{ pascal_singular }}Dto,
  ApiClientError,
  Update{{ pascal_singular }}Variables
> {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ id, data }: Update{{ pascal_singular }}Variables) =>
      put<{{ pascal_singular }}Dto>(`/api/{{ snake_plural }}/${id}`, data),
    onSuccess: (_data, variables) => {
      void queryClient.invalidateQueries({ queryKey: {{ camel_singular }}Keys.all });
      void queryClient.invalidateQueries({
        queryKey: {{ camel_singular }}Keys.detail(variables.id),
      });
    },
  });
}

export function useRemove{{ pascal_singular }}(): UseMutationResult<
  void,
  ApiClientError,
  number
> {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: number) => del(`/api/{{ snake_plural }}/${id}`),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: {{ camel_singular }}Keys.all });
    },
  });
}
