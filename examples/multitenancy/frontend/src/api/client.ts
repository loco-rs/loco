import { getToken } from "../auth/session";

export type HttpMethod = "GET" | "POST" | "PUT";

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
  const token = getToken();
  const headers: Record<string, string> = {};
  if (token) {
    headers.Authorization = `Bearer ${token}`;
  }
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

export function put<T>(path: string, body: unknown): Promise<T> {
  return request<T>("PUT", path, body);
}
