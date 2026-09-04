import { useState, type FormEvent } from "react";
import { Link, useOutletContext } from "react-router";
import { useClients, useCreateClient } from "../api/clients";
import { useDashboard } from "../api/dashboard";
import { hasPermission } from "../auth/permissions";
import type { SelectedWorkspace } from "../auth/session";
import type { WorkspaceOutletContext } from "../auth/workspace-context";
import { NoWorkspace } from "./Dashboard";

export function Clients() {
  const context = useOutletContext<WorkspaceOutletContext>();
  if (!context.selected) return <NoWorkspace onCreate={context.openWorkspaceCreator} />;
  return <ClientList workspace={context.selected} />;
}

function ClientList({ workspace }: { workspace: SelectedWorkspace }) {
  const clients = useClients(workspace);
  const dashboard = useDashboard(workspace.tenantId);
  const create = useCreateClient(workspace);
  const [name, setName] = useState("");
  const [email, setEmail] = useState("");
  const permissions = dashboard.data?.current_member.permissions;
  const canCreate = hasPermission(permissions, "clients:create");
  const canEdit = hasPermission(permissions, "clients:edit");
  function submit(event: FormEvent) { event.preventDefault(); create.mutate({ name: name.trim(), email: email.trim() }, { onSuccess: () => { setName(""); setEmail(""); } }); }
  return <section className="console-page"><header className="console-heading"><div><span className="eyebrow">Core resource</span><h1>Clients</h1><p>Manage client relationships for {workspace.tenantName}.</p></div></header><div className={`workspace-grid${canCreate ? "" : " single-column"}`}><section className="panel documents-panel"><div className="panel-heading"><div><span className="eyebrow">Client directory</span><h2>Your clients</h2></div><span className="count">{clients.data?.length ?? 0} clients</span></div>{clients.isLoading && <p className="muted">Loading clients…</p>}{clients.error && <p className="error" role="alert">{clients.error.message}</p>}<div className="resource-list">{clients.data?.map((client) => <article key={client.id}><span className="avatar">{client.name.charAt(0)}</span><div><strong>{client.name}</strong><small>{client.email}</small></div><div className="member-actions"><Link to={`/clients/${client.id}`}>View</Link>{canEdit && <Link className="edit" to={`/clients/${client.id}/edit`}>Edit</Link>}</div></article>)}</div>{clients.data?.length === 0 && <div className="empty-state">No clients exist in this tenant yet.</div>}</section>{canCreate && <form className="panel create-form" onSubmit={submit}><div className="create-form-heading"><span className="create-icon">+</span><div><span className="eyebrow">New client</span><h2>Add a client</h2></div></div><label htmlFor="client-name">Name</label><input id="client-name" value={name} onChange={(event) => setName(event.target.value)} required /><label htmlFor="client-email">Email</label><input id="client-email" type="email" value={email} onChange={(event) => setEmail(event.target.value)} required /><button className="primary" disabled={create.isPending}>{create.isPending ? "Creating…" : "Create client"}</button>{create.error && <p className="error" role="alert">{create.error.message}</p>}</form>}</div></section>;
}
