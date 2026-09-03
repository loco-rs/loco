import { useState, type FormEvent } from "react";
import { useOutletContext } from "react-router";
import { useDashboard } from "../api/dashboard";
import { useCreateDocument, useDocuments } from "../api/documents";
import type { SelectedWorkspace } from "../auth/session";
import type { WorkspaceOutletContext } from "../auth/workspace-context";

export function Documents() {
  const { selected, options, isLoading, error, openWorkspaceCreator } =
    useOutletContext<WorkspaceOutletContext>();

  if (isLoading) {
    return (
      <section className="panel page-state">
        <span className="status-dot" />
        <p>Loading your tenant workspaces…</p>
      </section>
    );
  }

  if (error) {
    return (
      <p className="error page-state" role="alert">
        {error.message}
      </p>
    );
  }

  if (!selected) {
    return (
      <section className="panel empty-state">
        <h1>No active workspace</h1>
        <p>Your account is not a member of a tenant with an active application.</p>
        <button
          className="primary"
          type="button"
          onClick={openWorkspaceCreator}
        >
          Create your first workspace
        </button>
      </section>
    );
  }

  const documentsWorkspace =
    options.find(
      (option) =>
        option.tenantId === selected.tenantId &&
        option.applicationName === "Documents",
    ) ?? selected;

  return (
    <section className="documents-page">
      <header className="workspace-heading">
        <div>
          <span className="eyebrow">Tenant workspace</span>
          <h1>Documents</h1>
          <p>Create and manage records inside an explicitly scoped workspace.</p>
        </div>
      </header>

      <DocumentsWorkspace
        key={`${documentsWorkspace.tenantId}:${documentsWorkspace.applicationId}`}
        workspace={documentsWorkspace}
      />
    </section>
  );
}

function DocumentsWorkspace({ workspace }: { workspace: SelectedWorkspace }) {
  const documents = useDocuments(workspace);
  const createDocument = useCreateDocument(workspace);
  const dashboard = useDashboard(workspace.tenantId);
  const [title, setTitle] = useState("");
  const canCreate = dashboard.data?.current_member.permissions.some(
    (permission) =>
      permission.application_id === workspace.applicationId &&
      permission.key === "documents:create",
  );

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

  if (dashboard.isLoading) {
    return <div className="panel page-state">Loading document access…</div>;
  }
  if (dashboard.error) {
    return <p className="error" role="alert">{dashboard.error.message}</p>;
  }

  return (
    <div className={`workspace-grid${canCreate ? "" : " single-column"}`}>
      <div className="panel documents-panel">
        <div className="panel-heading">
          <div>
            <span className="eyebrow">Document library</span>
            <h2>Your documents</h2>
          </div>
          <span className="count">{documents.data?.length ?? 0} records</span>
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
                <small>Document #{document.id} · Tenant #{document.tenant_id}</small>
              </div>
              <span className="document-scope">Scoped</span>
            </li>
          ))}
        </ul>
      </div>

      {canCreate ? <form className="panel create-form" onSubmit={handleSubmit}>
        <div className="create-form-heading">
          <span className="create-icon">+</span>
          <div>
            <span className="eyebrow">New record</span>
            <h2>Create document</h2>
          </div>
        </div>
        <p className="form-description">
          Add a document to the {workspace.tenantName} workspace.
        </p>
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
      </form> : (
        <aside className="panel access-note">
          <span className="eyebrow">Read-only access</span>
          <h2>Read-only permission</h2>
          <p>Your current role can read Documents but cannot create them.</p>
        </aside>
      )}
    </div>
  );
}
