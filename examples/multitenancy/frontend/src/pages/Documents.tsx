import { useState, type FormEvent } from "react";
import { Link } from "react-router";
import { useCreateDocument, useDocuments } from "../api/documents";
import { loadAccess } from "../auth/access";

export function Documents() {
  const access = loadAccess();
  if (!access) {
    return null;
  }

  return <DocumentsWorkspace access={access} />;
}

function DocumentsWorkspace({ access }: { access: NonNullable<ReturnType<typeof loadAccess>> }) {
  const documents = useDocuments(access);
  const createDocument = useCreateDocument(access);
  const [title, setTitle] = useState("");

  function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const trimmed = title.trim();
    if (trimmed.length === 0) {
      return;
    }

    createDocument.mutate(
      { title: trimmed },
      { onSuccess: () => setTitle("") },
    );
  }

  return (
    <section>
      <div className="workspace-heading">
        <div>
          <span className="eyebrow">Tenant {access.tenantId}</span>
          <h1>Documents</h1>
          <p>Application {access.applicationId} · isolated at query time</p>
        </div>
        <Link className="context-link" to="/access">Change context</Link>
      </div>

      <div className="workspace-grid">
        <div className="panel">
          <div className="panel-heading">
            <h2>Tenant records</h2>
            <span className="count">{documents.data?.length ?? 0}</span>
          </div>
          {documents.isLoading && <p className="muted">Loading documents…</p>}
          {documents.error && <p className="error" role="alert">{documents.error.message}</p>}
          {documents.data?.length === 0 && (
            <div className="empty-state">No documents exist in this tenant yet.</div>
          )}
          <ul className="document-list">
            {documents.data?.map((document) => (
              <li key={document.id}>
                <span className="document-icon">D</span>
                <div>
                  <strong>{document.title}</strong>
                  <small>Record #{document.id} · tenant {document.tenant_id}</small>
                </div>
              </li>
            ))}
          </ul>
        </div>

        <form className="panel create-form" onSubmit={handleSubmit}>
          <span className="eyebrow">Permission: documents:create</span>
          <h2>Add a document</h2>
          <label htmlFor="document-title">Title</label>
          <input
            id="document-title"
            value={title}
            onChange={(event) => setTitle(event.target.value)}
            placeholder="Quarterly launch plan"
            required
          />
          <button className="primary" type="submit" disabled={createDocument.isPending}>
            {createDocument.isPending ? "Creating…" : "Create document"}
          </button>
          {createDocument.error && (
            <p className="error" role="alert">{createDocument.error.message}</p>
          )}
          <p className="hint">
            The browser never sends <code>tenant_id</code> in the body. Loco
            assigns it from the trusted route context with <code>set_tenant</code>.
          </p>
        </form>
      </div>
    </section>
  );
}
