import { useState, type FormEvent } from "react";
import { useNavigate } from "react-router";
import { clearAccess, loadAccess, saveAccess } from "../auth/access";

export function Access() {
  const navigate = useNavigate();
  const current = loadAccess();
  const [apiKey, setApiKey] = useState(current?.apiKey ?? "");
  const [tenantId, setTenantId] = useState(String(current?.tenantId ?? 1));
  const [applicationId, setApplicationId] = useState(
    String(current?.applicationId ?? 1),
  );
  const [error, setError] = useState<string | null>(null);

  function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const tenant = Number(tenantId);
    const application = Number(applicationId);

    if (
      apiKey.trim().length === 0 ||
      !Number.isInteger(tenant) ||
      tenant <= 0 ||
      !Number.isInteger(application) ||
      application <= 0
    ) {
      setError("Enter an API key and positive tenant and application IDs.");
      return;
    }

    saveAccess({
      apiKey: apiKey.trim(),
      tenantId: tenant,
      applicationId: application,
    });
    navigate("/documents");
  }

  function handleClear() {
    clearAccess();
    setApiKey("");
    setError(null);
  }

  return (
    <section className="access-grid">
      <div className="intro-card">
        <span className="eyebrow">Request scope</span>
        <h1>Choose your tenant context</h1>
        <p>
          Every document request carries an authenticated user and explicit
          tenant/application pair. The API verifies membership, subscription,
          role, and permission before touching tenant-owned rows.
        </p>
        <ol className="scope-list">
          <li>User API key</li>
          <li>Tenant membership</li>
          <li>Active application subscription</li>
          <li>Role permission</li>
        </ol>
      </div>

      <form className="panel access-form" onSubmit={handleSubmit}>
        <div>
          <span className="eyebrow">Access context</span>
          <h2>Connect to the demo</h2>
        </div>
        <label htmlFor="api-key">User API key</label>
        <input
          id="api-key"
          type="password"
          autoComplete="off"
          placeholder="lo-…"
          value={apiKey}
          onChange={(event) => setApiKey(event.target.value)}
        />
        <div className="field-row">
          <div>
            <label htmlFor="tenant-id">Tenant ID</label>
            <input
              id="tenant-id"
              inputMode="numeric"
              value={tenantId}
              onChange={(event) => setTenantId(event.target.value)}
            />
          </div>
          <div>
            <label htmlFor="application-id">Application ID</label>
            <input
              id="application-id"
              inputMode="numeric"
              value={applicationId}
              onChange={(event) => setApplicationId(event.target.value)}
            />
          </div>
        </div>
        {error && <p className="error" role="alert">{error}</p>}
        <div className="button-row">
          <button className="primary" type="submit">Open workspace</button>
          {current && (
            <button className="secondary" type="button" onClick={handleClear}>
              Clear saved context
            </button>
          )}
        </div>
        <p className="hint">
          Run <code>cargo loco db seed</code> and use the sample values from the
          example README for a ready-made workspace.
        </p>
      </form>
    </section>
  );
}
