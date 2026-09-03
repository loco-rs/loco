import { useEffect, useState, type FormEvent } from "react";
import { Link, useOutletContext, useParams } from "react-router";
import { useClient, useUpdateClient } from "../api/clients";
import { useDashboard } from "../api/dashboard";
import { hasPermission } from "../auth/permissions";
import type { WorkspaceOutletContext } from "../auth/workspace-context";
import { NoWorkspace } from "./Dashboard";

export function ClientManagement({ edit = false }: { edit?: boolean }) {
  const context = useOutletContext<WorkspaceOutletContext>();
  const id = Number(useParams().clientId);
  if (!context.selected) return <NoWorkspace onCreate={context.openWorkspaceCreator} />;
  const workspace = context.selected;
  const client = useClient(workspace, id);
  const dashboard = useDashboard(workspace.tenantId);
  const update = useUpdateClient(workspace, id);
  const [name, setName] = useState(""); const [email, setEmail] = useState("");
  useEffect(() => { if (client.data) { setName(client.data.name); setEmail(client.data.email); } }, [client.data]);
  const canEdit = hasPermission(dashboard.data?.current_member.permissions, "clients:edit");
  function submit(event: FormEvent) { event.preventDefault(); update.mutate({ name: name.trim(), email: email.trim() }); }
  if (client.isLoading) return <div className="panel page-state">Loading client…</div>;
  if (client.error || !client.data) return <p className="error" role="alert">{client.error?.message ?? "Client not found."}</p>;
  return <section className="console-page resource-page"><header className="console-heading"><div><span className="eyebrow">Client #{client.data.id}</span><h1>{edit ? "Edit client" : client.data.name}</h1><p>{client.data.email}</p></div><Link className="secondary member-page-link" to="/clients">← Back to clients</Link></header><div className="panel resource-detail">{edit && canEdit ? <form className="resource-form" onSubmit={submit}><label htmlFor="edit-client-name">Name</label><input id="edit-client-name" value={name} onChange={(event) => setName(event.target.value)} required /><label htmlFor="edit-client-email">Email</label><input id="edit-client-email" type="email" value={email} onChange={(event) => setEmail(event.target.value)} required /><button className="primary" disabled={update.isPending}>{update.isPending ? "Saving…" : "Save client"}</button></form> : <><span className="member-profile-avatar">{client.data.name.charAt(0)}</span><div><span className="eyebrow">Client</span><h2>{client.data.name}</h2><p>{client.data.email}</p></div>{canEdit && <Link className="primary member-page-link" to={`/clients/${id}/edit`}>Edit client</Link>}</>}</div></section>;
}
