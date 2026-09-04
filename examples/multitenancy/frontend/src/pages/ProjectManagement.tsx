import { useEffect, useState, type FormEvent } from "react";
import { Link, useOutletContext, useParams } from "react-router";
import { useClients } from "../api/clients";
import { useDashboard } from "../api/dashboard";
import { useProject, useUpdateProject } from "../api/projects";
import { hasPermission } from "../auth/permissions";
import type { WorkspaceOutletContext } from "../auth/workspace-context";
import { NoWorkspace } from "./Dashboard";

export function ProjectManagement({ edit = false }: { edit?: boolean }) {
  const context = useOutletContext<WorkspaceOutletContext>();
  const id = Number(useParams().projectId);
  if (!context.selected) {
    return <NoWorkspace onCreate={context.openWorkspaceCreator} />;
  }
  const workspace = context.selected;
  const project = useProject(workspace, id);
  const clients = useClients(workspace);
  const dashboard = useDashboard(workspace.tenantId);
  const update = useUpdateProject(workspace, id);
  const [clientId, setClientId] = useState("");
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");

  useEffect(() => {
    if (project.data) {
      setClientId(String(project.data.client_id));
      setName(project.data.name);
      setDescription(project.data.description);
    }
  }, [project.data]);

  const permissions = dashboard.data?.current_member.permissions;
  const canEdit =
    hasPermission(permissions, "projects:edit") &&
    hasPermission(permissions, "clients:view");

  function submit(event: FormEvent) {
    event.preventDefault();
    if (!clientId) return;
    update.mutate({
      client_id: Number(clientId),
      name: name.trim(),
      description: description.trim(),
    });
  }

  if (project.isLoading) {
    return <div className="panel page-state">Loading project…</div>;
  }
  if (project.error || !project.data) {
    return <p className="error" role="alert">{project.error?.message ?? "Project not found."}</p>;
  }

  return (
    <section className="console-page resource-page">
      <header className="console-heading">
        <div>
          <span className="eyebrow">Project #{project.data.id}</span>
          <h1>{edit ? "Edit project" : project.data.name}</h1>
          <p>{project.data.description}</p>
        </div>
        <Link className="secondary member-page-link" to="/projects">← Back to projects</Link>
      </header>
      <div className="panel resource-detail">
        {edit && canEdit ? (
          <form className="resource-form" onSubmit={submit}>
            <label htmlFor="edit-project-client">Client</label>
            <select id="edit-project-client" value={clientId} onChange={(event) => setClientId(event.target.value)} required>
              {clients.data?.map((client) => (
                <option value={client.id} key={client.id}>{client.name}</option>
              ))}
            </select>
            {clients.error && <p className="error" role="alert">{clients.error.message}</p>}
            <label htmlFor="edit-project-name">Name</label>
            <input id="edit-project-name" value={name} onChange={(event) => setName(event.target.value)} required />
            <label htmlFor="edit-project-description">Description</label>
            <textarea id="edit-project-description" value={description} onChange={(event) => setDescription(event.target.value)} required />
            {update.error && <p className="error" role="alert">{update.error.message}</p>}
            <button className="primary" disabled={update.isPending || clients.isLoading}>{update.isPending ? "Saving…" : "Save project"}</button>
          </form>
        ) : (
          <>
            <span className="document-icon large">P</span>
            <div>
              <span className="eyebrow">Client</span>
              <h2>{project.data.name}</h2>
              <p className="project-client-name">{project.data.client_name}</p>
              <p>{project.data.description}</p>
            </div>
            {canEdit && <Link className="primary member-page-link" to={`/projects/${id}/edit`}>Edit project</Link>}
          </>
        )}
      </div>
    </section>
  );
}
