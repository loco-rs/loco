import { useState, type FormEvent } from "react";
import { Link, useNavigate } from "react-router";
import { registerAccount } from "../api/auth";
import { ApiClientError } from "../api/client";
import { setToken } from "../auth/session";

export function Register() {
  const navigate = useNavigate();
  const [name, setName] = useState("");
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [pending, setPending] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setPending(true);
    setError(null);

    try {
      const session = await registerAccount({
        name,
        email,
        password,
      });
      setToken(session.token);
      navigate("/documents", { state: { createWorkspace: true } });
    } catch (reason) {
      setError(
        reason instanceof ApiClientError
          ? reason.message
          : "Unable to create your account",
      );
    } finally {
      setPending(false);
    }
  }

  return (
    <section className="auth-layout">
      <div className="intro-card">
        <span className="eyebrow">New account</span>
        <h1>Start building with Loco.</h1>
        <p>
          Create your account first. We will help you set up your first tenant
          workspace immediately after registration.
        </p>
      </div>
      <form className="panel auth-form" onSubmit={handleSubmit}>
        <h2>Create account</h2>
        <label htmlFor="name">Your name</label>
        <input
          id="name"
          autoComplete="name"
          value={name}
          onChange={(event) => setName(event.target.value)}
          required
        />
        <label htmlFor="email">Email</label>
        <input
          id="email"
          type="email"
          autoComplete="email"
          value={email}
          onChange={(event) => setEmail(event.target.value)}
          required
        />
        <label htmlFor="password">Password</label>
        <input
          id="password"
          type="password"
          minLength={8}
          autoComplete="new-password"
          value={password}
          onChange={(event) => setPassword(event.target.value)}
          required
        />
        {error && <p className="error" role="alert">{error}</p>}
        <button className="primary" type="submit" disabled={pending}>
          {pending ? "Creating account…" : "Register and continue"}
        </button>
        <p className="hint">
          Already registered? <Link to="/login">Log in</Link>.
        </p>
      </form>
    </section>
  );
}
