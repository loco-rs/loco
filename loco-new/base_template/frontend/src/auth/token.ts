const TOKEN_KEY = "auth_token";

export function getToken(): string | null {
  return window.localStorage.getItem(TOKEN_KEY);
}

export function setToken(t: string): void {
  window.localStorage.setItem(TOKEN_KEY, t);
}

export function clearToken(): void {
  window.localStorage.removeItem(TOKEN_KEY);
}
