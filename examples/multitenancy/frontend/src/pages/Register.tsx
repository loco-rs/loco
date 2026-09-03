import { useState, type FormEvent } from "react";
import { Link, useNavigate } from "react-router";
import { registerTenant } from "../api/auth";
import { ApiClientError } from "../api/client";
import { saveWorkspace, setToken } from "../auth/session";
import { tenantSlug } from "../auth/tenant";

export function Register() {
  const navigate = useNavigate();
  const [name, setName] = useState("");
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [tenantName, setTenantName] = useState("");
  const [pending, setPending] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setPending(true);
    setError(null);

    try {
      const session = await registerTenant({
        name,
        email,
        password,
        tenant_name: tenantName,
        tenant_slug: tenantSlug(tenantName),
      });
      setToken(session.token);
      saveWorkspace({
        tenantId: session.tenant_id,
        tenantName: session.tenant_name,
        applicationId: session.application_id,
        applicationName: session.application_name,
      });
      navigate("/documents");
    } catch (reason) {
      setError(
        reason instanceof ApiClientError
          ? reason.message
          : "Unable to create the tenant workspace",
      );
    } finally {
      setPending(false);
    }
  }

  return (
    <section className="auth-layout">
      <div className="intro-card">
        <span className="eyebrow">New workspace</span>
        <h1>Start with your own tenant.</h1>
        <p>
          Registration atomically creates your account, tenant membership,
          owner role, active Documents subscription, and default permissions.
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
        <label htmlFor="tenant-name">Tenant name</label>
        <input
          id="tenant-name"
          value={tenantName}
          onChange={(event) => setTenantName(event.target.value)}
          required
        />
        {error && <p className="error" role="alert">{error}</p>}
        <button className="primary" type="submit" disabled={pending}>
          {pending ? "Creating workspace…" : "Register and continue"}
        </button>
        <p className="hint">
          Already registered? <Link to="/login">Log in</Link>.
        </p>
      </form>
    </section>
  );
}
