import { loadAccess } from "../auth/access";

export type HttpMethod = "GET" | "POST";

interface LocoErrorBody {
  error?: string;
  description?: string;
  errors?: unknown;
}

export class ApiClientError extends Error {
  readonly status: number;
  readonly body: LocoErrorBody | null;

  constructor(status: number, body: LocoErrorBody | null) {
    super(body?.description ?? `Request failed with status ${status}`);
    this.name = "ApiClientError";
    this.status = status;
    this.body = body;
  }
}

async function parseErrorBody(response: Response): Promise<LocoErrorBody | null> {
  try {
    const value: unknown = await response.json();
    return typeof value === "object" && value !== null
      ? (value as LocoErrorBody)
      : null;
  } catch {
    return null;
  }
}

export async function request<T>(
  method: HttpMethod,
  path: string,
  body?: unknown,
): Promise<T> {
  const access = loadAccess();
  if (!access) {
    throw new ApiClientError(0, {
      error: "missing_access_context",
      description: "Configure an API key, tenant, and application first",
    });
  }

  const headers: Record<string, string> = {
    Authorization: `Bearer ${access.apiKey}`,
  };
  if (body !== undefined) {
    headers["Content-Type"] = "application/json";
  }

  const response = await fetch(path, {
    method,
    headers,
    credentials: "same-origin",
    body: body === undefined ? undefined : JSON.stringify(body),
  });

  if (response.ok) {
    return (await response.json()) as T;
  }

  throw new ApiClientError(response.status, await parseErrorBody(response));
}

export function get<T>(path: string): Promise<T> {
  return request<T>("GET", path);
}

export function post<T>(path: string, body: unknown): Promise<T> {
  return request<T>("POST", path, body);
}
