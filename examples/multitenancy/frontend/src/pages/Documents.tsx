import { useState, type FormEvent } from "react";
import { Link, useOutletContext } from "react-router";
import { useDashboard } from "../api/dashboard";
import { useCreateDocument, useDocuments } from "../api/documents";
import { hasPermission } from "../auth/permissions";
import type { SelectedWorkspace } from "../auth/session";
import type { WorkspaceOutletContext } from "../auth/workspace-context";
import { NoWorkspace } from "./Dashboard";

export function Documents() {
  const context = useOutletContext<WorkspaceOutletContext>();
  if (context.isLoading) return <div className="panel page-state">Loading your workspace…</div>;
  if (context.error) return <p className="error" role="alert">{context.error.message}</p>;
  if (!context.selected) return <NoWorkspace onCreate={context.openWorkspaceCreator} />;
  return <DocumentList workspace={context.selected} />;
}

function DocumentList({ workspace }: { workspace: SelectedWorkspace }) {
  const documents = useDocuments(workspace);
  const createDocument = useCreateDocument(workspace);
  const dashboard = useDashboard(workspace.tenantId);
  const [title, setTitle] = useState("");
  const permissions = dashboard.data?.current_member.permissions;
  const canCreate = hasPermission(permissions, "documents:create");
  const canEdit = hasPermission(permissions, "documents:edit");

  function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const value = title.trim();
    if (!value) return;
    createDocument.mutate({ title: value }, { onSuccess: () => setTitle("") });
  }

  if (dashboard.isLoading) return <div className="panel page-state">Loading document access…</div>;
  if (dashboard.error) return <p className="error" role="alert">{dashboard.error.message}</p>;

  return (
    <section className="console-page">
      <header className="console-heading"><div><span className="eyebrow">Core resource</span><h1>Documents</h1><p>Create, view, and edit tenant-owned documents in {workspace.tenantName}.</p></div></header>
      <div className={`workspace-grid${canCreate ? "" : " single-column"}`}>
        <section className="panel documents-panel">
          <div className="panel-heading"><div><span className="eyebrow">Document library</span><h2>Your documents</h2></div><span className="count">{documents.data?.length ?? 0} records</span></div>
          {documents.isLoading && <p className="muted">Loading documents…</p>}
          {documents.error && <p className="error" role="alert">{documents.error.message}</p>}
          <div className="resource-list">
            {documents.data?.map((document) => <article key={document.id}><span className="document-icon">D</span><div><strong>{document.title}</strong><small>Document #{document.id}</small></div><div className="member-actions"><Link to={`/documents/${document.id}`}>View</Link>{canEdit && <Link className="edit" to={`/documents/${document.id}/edit`}>Edit</Link>}</div></article>)}
          </div>
          {documents.data?.length === 0 && <div className="empty-state">No documents exist in this tenant yet.</div>}
        </section>
        {canCreate && <form className="panel create-form" onSubmit={submit}><div className="create-form-heading"><span className="create-icon">+</span><div><span className="eyebrow">New record</span><h2>Create document</h2></div></div><label htmlFor="document-title">Title</label><input id="document-title" value={title} onChange={(event) => setTitle(event.target.value)} placeholder="Quarterly launch plan" required /><button className="primary" type="submit" disabled={createDocument.isPending}>{createDocument.isPending ? "Creating…" : "Create document"}</button>{createDocument.error && <p className="error" role="alert">{createDocument.error.message}</p>}</form>}
      </div>
    </section>
  );
}
