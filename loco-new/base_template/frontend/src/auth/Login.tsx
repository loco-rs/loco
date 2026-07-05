import { useState } from "react";
import type { FormEvent } from "react";
import { useNavigate } from "react-router";
import { ApiClientError, post } from "../api/client";
import { setToken } from "./token";

interface LoginResponse {
  token: string;
  pid: string;
  name: string;
  is_verified: boolean;
}

export function Login() {
  const navigate = useNavigate();
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [isPending, setIsPending] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function handleSubmit(e: FormEvent<HTMLFormElement>) {
    e.preventDefault();
    setError(null);
    setIsPending(true);
    try {
      const res = await post<LoginResponse>("/api/auth/login", {
        email,
        password,
      });
      setToken(res.token);
      navigate("/");
    } catch (err) {
      setError(
        err instanceof ApiClientError ? err.message : "Failed to log in",
      );
    } finally {
      setIsPending(false);
    }
  }

  return (
    <div>
      <h1>Log in</h1>
      <form onSubmit={handleSubmit}>
        <div>
          <label htmlFor="email">Email</label>
          <input
            id="email"
            name="email"
            type="email"
            autoComplete="email"
            required
            value={email}
            onChange={(e) => setEmail(e.target.value)}
          />
        </div>
        <div>
          <label htmlFor="password">Password</label>
          <input
            id="password"
            name="password"
            type="password"
            autoComplete="current-password"
            required
            value={password}
            onChange={(e) => setPassword(e.target.value)}
          />
        </div>
        <button type="submit" disabled={isPending}>
          {isPending ? "Logging in…" : "Log in"}
        </button>
      </form>
      {error && <p role="alert">{error}</p>}
      <p>
        Don&apos;t have an account? You must register via{" "}
        <code>POST /api/auth/register</code> and verify your email before you
        can log in.
      </p>
    </div>
  );
}
