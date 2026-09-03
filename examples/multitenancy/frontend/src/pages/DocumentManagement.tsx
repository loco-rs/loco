import { useEffect, useState, type FormEvent } from "react";
import { Link, useOutletContext, useParams } from "react-router";
import { useDashboard } from "../api/dashboard";
import { useDocument, useUpdateDocument } from "../api/documents";
import { hasPermission } from "../auth/permissions";
import type { SelectedWorkspace } from "../auth/session";
import type { WorkspaceOutletContext } from "../auth/workspace-context";
import { NoWorkspace } from "./Dashboard";

export function DocumentManagement({ edit = false }: { edit?: boolean }) {
  const context = useOutletContext<WorkspaceOutletContext>();
  const id = Number(useParams().documentId);
  if (!context.selected) return <NoWorkspace onCreate={context.openWorkspaceCreator} />;
  return <DocumentPage workspace={context.selected} id={id} edit={edit} />;
}

function DocumentPage({ workspace, id, edit }: { workspace: SelectedWorkspace; id: number; edit: boolean }) {
  const document = useDocument(workspace, id);
  const dashboard = useDashboard(workspace.tenantId);
  const update = useUpdateDocument(workspace, id);
  const [title, setTitle] = useState("");
  useEffect(() => { if (document.data) setTitle(document.data.title); }, [document.data]);
  const canEdit = hasPermission(dashboard.data?.current_member.permissions, "documents:edit");
  function submit(event: FormEvent) { event.preventDefault(); if (title.trim()) update.mutate({ title: title.trim() }); }
  if (document.isLoading || dashboard.isLoading) return <div className="panel page-state">Loading document…</div>;
  if (document.error || !document.data) return <p className="error" role="alert">{document.error?.message ?? "Document not found."}</p>;
  return <section className="console-page resource-page"><header className="console-heading"><div><span className="eyebrow">Document #{document.data.id}</span><h1>{edit ? "Edit document" : document.data.title}</h1><p>Tenant-owned record in {workspace.tenantName}.</p></div><Link className="secondary member-page-link" to="/documents">← Back to documents</Link></header><div className="panel resource-detail">{edit && canEdit ? <form className="resource-form" onSubmit={submit}><label htmlFor="edit-document-title">Title</label><input id="edit-document-title" value={title} onChange={(event) => setTitle(event.target.value)} required />{update.error && <p className="error" role="alert">{update.error.message}</p>}<button className="primary" disabled={update.isPending}>{update.isPending ? "Saving…" : "Save document"}</button></form> : <><span className="document-icon large">D</span><div><span className="eyebrow">Title</span><h2>{document.data.title}</h2><p className="muted">Updated {new Date(document.data.updated_at).toLocaleDateString()}</p></div>{canEdit && <Link className="primary member-page-link" to={`/documents/${id}/edit`}>Edit document</Link>}</>}</div></section>;
}
