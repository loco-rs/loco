import { useState, type FormEvent } from "react";
import { useCreateWorkspace, useWorkspaces } from "../api/auth";
import { useCreateDocument, useDocuments } from "../api/documents";
import {
  loadWorkspace,
  saveWorkspace,
  type SelectedWorkspace,
} from "../auth/session";
import { tenantSlug } from "../auth/tenant";
import type { Workspace } from "../bindings/Workspace";

export function Documents() {
  const workspaces = useWorkspaces();
  const [savedWorkspace, setSavedWorkspace] = useState(loadWorkspace);
  const [workspaceCreatorOpen, setWorkspaceCreatorOpen] = useState(false);

  if (workspaces.isLoading) {
    return (
      <section className="panel page-state">
        <span className="status-dot" />
        <p>Loading your tenant workspaces…</p>
      </section>
    );
  }

  if (workspaces.error) {
    return (
      <p className="error page-state" role="alert">
        {workspaces.error.message}
      </p>
    );
  }

  const options: SelectedWorkspace[] =
    workspaces.data?.flatMap((workspace) =>
      workspace.applications.map((application) => ({
        tenantId: workspace.tenant_id,
        tenantName: workspace.tenant_name,
        applicationId: application.id,
        applicationName: application.name,
      })),
    ) ?? [];

  const selected =
    options.find(
      (option) =>
        option.tenantId === savedWorkspace?.tenantId &&
        option.applicationId === savedWorkspace.applicationId,
    ) ?? options[0];

  function activateWorkspace(workspace: Workspace) {
    const application = workspace.applications[0];
    if (!application) {
      return;
    }

    const next = {
      tenantId: workspace.tenant_id,
      tenantName: workspace.tenant_name,
      applicationId: application.id,
      applicationName: application.name,
    };
    saveWorkspace(next);
    setSavedWorkspace(next);
    setWorkspaceCreatorOpen(false);
  }

  if (!selected) {
    return (
      <section className="panel empty-state">
        <h1>No active workspace</h1>
        <p>Your account is not a member of a tenant with an active application.</p>
        <button
          className="primary"
          type="button"
          onClick={() => setWorkspaceCreatorOpen(true)}
        >
          Create your first workspace
        </button>
        {workspaceCreatorOpen && (
          <WorkspaceCreator
            onClose={() => setWorkspaceCreatorOpen(false)}
            onCreated={activateWorkspace}
          />
        )}
      </section>
    );
  }

  function selectWorkspace(value: string) {
    const next = options.find(
      (option) => `${option.tenantId}:${option.applicationId}` === value,
    );
    if (next) {
      saveWorkspace(next);
      setSavedWorkspace(next);
    }
  }

  return (
    <section className="documents-page">
      <header className="workspace-heading">
        <div>
          <span className="eyebrow">Tenant workspace</span>
          <h1>Documents</h1>
          <p>Create and manage records inside an explicitly scoped workspace.</p>
        </div>
        <div className="workspace-actions">
          <label className="workspace-picker">
            <span>Working in</span>
            <select
              value={`${selected.tenantId}:${selected.applicationId}`}
              onChange={(event) => selectWorkspace(event.target.value)}
            >
              {options.map((option) => (
                <option
                  key={`${option.tenantId}:${option.applicationId}`}
                  value={`${option.tenantId}:${option.applicationId}`}
                >
                  {option.tenantName} · {option.applicationName}
                </option>
              ))}
            </select>
          </label>
          <button
            className="secondary new-workspace-button"
            type="button"
            onClick={() => setWorkspaceCreatorOpen(true)}
          >
            <span aria-hidden="true">+</span> New workspace
          </button>
        </div>
      </header>

      <div className="workspace-context" aria-label="Current tenant context">
        <div>
          <span>Tenant</span>
          <strong>{selected.tenantName}</strong>
        </div>
        <div>
          <span>Application</span>
          <strong>{selected.applicationName}</strong>
        </div>
        <div className="scope-status">
          <span className="status-dot" />
          <strong>Tenant scope active</strong>
        </div>
      </div>

      <DocumentsWorkspace
        key={`${selected.tenantId}:${selected.applicationId}`}
        workspace={selected}
      />

      {workspaceCreatorOpen && (
        <WorkspaceCreator
          onClose={() => setWorkspaceCreatorOpen(false)}
          onCreated={activateWorkspace}
        />
      )}
    </section>
  );
}

function WorkspaceCreator({
  onClose,
  onCreated,
}: {
  onClose: () => void;
  onCreated: (workspace: Workspace) => void;
}) {
  const createWorkspace = useCreateWorkspace();
  const [name, setName] = useState("");
  const [validationError, setValidationError] = useState<string | null>(null);

  function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const slug = tenantSlug(name);
    if (!slug) {
      setValidationError("Use at least one letter or number in the workspace name.");
      return;
    }

    createWorkspace.mutate(
      { tenant_name: name.trim(), tenant_slug: slug },
      { onSuccess: onCreated },
    );
  }

  return (
    <div
      className="workspace-modal-backdrop"
      role="presentation"
      onMouseDown={onClose}
    >
      <section
        className="panel workspace-modal"
        role="dialog"
        aria-modal="true"
        aria-labelledby="workspace-modal-title"
        onKeyDown={(event) => {
          if (event.key === "Escape") {
            onClose();
          }
        }}
        onMouseDown={(event) => event.stopPropagation()}
      >
        <button
          className="modal-close"
          type="button"
          aria-label="Close workspace form"
          onClick={onClose}
        >
          ×
        </button>
        <span className="eyebrow">New tenant</span>
        <h2 id="workspace-modal-title">Create a workspace</h2>
        <p className="form-description">
          You will become the owner with permission to read and create documents.
        </p>
        <form onSubmit={handleSubmit}>
          <label htmlFor="workspace-name">Workspace name</label>
          <input
            id="workspace-name"
            autoFocus
            minLength={2}
            maxLength={100}
            value={name}
            onChange={(event) => {
              setName(event.target.value);
              setValidationError(null);
            }}
            placeholder="Research team"
            required
          />
          <p className="hint">The workspace slug is generated automatically.</p>
          {(validationError || createWorkspace.error) && (
            <p className="error" role="alert">
              {validationError ?? createWorkspace.error?.message}
            </p>
          )}
          <div className="modal-actions">
            <button className="secondary" type="button" onClick={onClose}>
              Cancel
            </button>
            <button
              className="primary"
              type="submit"
              disabled={createWorkspace.isPending}
            >
              {createWorkspace.isPending ? "Creating…" : "Create workspace"}
            </button>
          </div>
        </form>
      </section>
    </div>
  );
}

function DocumentsWorkspace({ workspace }: { workspace: SelectedWorkspace }) {
  const documents = useDocuments(workspace);
  const createDocument = useCreateDocument(workspace);
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
    <div className="workspace-grid">
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

      <form className="panel create-form" onSubmit={handleSubmit}>
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
        <div className="scope-note">
          <span className="status-dot" />
          <p>
            <strong>Tenant-safe by default</strong>
            The browser sends only the title. Loco assigns the authenticated
            tenant with <code>set_tenant</code>.
          </p>
        </div>
      </form>
    </div>
  );
}
