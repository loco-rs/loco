import { useState, type FormEvent } from "react";
import { Link, useNavigate } from "react-router";
import { login } from "../api/auth";
import { ApiClientError } from "../api/client";
import { clearWorkspace, setToken } from "../auth/session";

export function Login() {
  const navigate = useNavigate();
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [pending, setPending] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setPending(true);
    setError(null);

    try {
      const session = await login({ email, password });
      setToken(session.token);
      clearWorkspace();
      navigate("/dashboard");
    } catch (reason) {
      setError(reason instanceof ApiClientError ? reason.message : "Unable to sign in");
    } finally {
      setPending(false);
    }
  }

  return (
    <section className="auth-layout">
      <div className="intro-card">
        <span className="eyebrow">Welcome back</span>
        <h1>One account, many tenants.</h1>
        <p>
          Sign in once, then select any tenant workspace where you are a
          member. Core features appear according to your permissions.
        </p>
      </div>
      <form className="panel auth-form" onSubmit={handleSubmit}>
        <h2>Login</h2>
        <div className="auth-fields">
          <div className="auth-field">
            <label htmlFor="email">Email</label>
            <input
              id="email"
              type="email"
              autoComplete="email"
              value={email}
              onChange={(event) => setEmail(event.target.value)}
              required
            />
          </div>
          <div className="auth-field">
            <label htmlFor="password">Password</label>
            <input
              id="password"
              type="password"
              autoComplete="current-password"
              value={password}
              onChange={(event) => setPassword(event.target.value)}
              required
            />
          </div>
        </div>
        {error && <p className="error" role="alert">{error}</p>}
        <button className="primary auth-submit" type="submit" disabled={pending}>
          {pending ? "Signing in…" : "Login"}
        </button>
        <p className="hint">
          New here? <Link to="/register">Create an account</Link>.
        </p>
      </form>
    </section>
  );
}
