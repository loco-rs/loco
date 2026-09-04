import { useState, type FormEvent } from "react";
import { Link, useOutletContext } from "react-router";
import { useClients } from "../api/clients";
import { useDashboard } from "../api/dashboard";
import { useCreateProject, useProjects } from "../api/projects";
import { hasPermission } from "../auth/permissions";
import type { SelectedWorkspace } from "../auth/session";
import type { WorkspaceOutletContext } from "../auth/workspace-context";
import { NoWorkspace } from "./Dashboard";

export function Projects() {
  const context = useOutletContext<WorkspaceOutletContext>();
  if (!context.selected) {
    return <NoWorkspace onCreate={context.openWorkspaceCreator} />;
  }
  return <ProjectList workspace={context.selected} />;
}

function ProjectList({ workspace }: { workspace: SelectedWorkspace }) {
  const projects = useProjects(workspace);
  const clients = useClients(workspace);
  const dashboard = useDashboard(workspace.tenantId);
  const create = useCreateProject(workspace);
  const [clientId, setClientId] = useState("");
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const permissions = dashboard.data?.current_member.permissions;
  const canCreate =
    hasPermission(permissions, "projects:create") &&
    hasPermission(permissions, "clients:view");
  const canEdit = hasPermission(permissions, "projects:edit");

  function submit(event: FormEvent) {
    event.preventDefault();
    if (!clientId) return;
    create.mutate(
      {
        client_id: Number(clientId),
        name: name.trim(),
        description: description.trim(),
      },
      {
        onSuccess: () => {
          setClientId("");
          setName("");
          setDescription("");
        },
      },
    );
  }

  return (
    <section className="console-page">
      <header className="console-heading">
        <div>
          <span className="eyebrow">Core resource</span>
          <h1>Projects</h1>
          <p>Plan and track client projects in {workspace.tenantName}.</p>
        </div>
      </header>
      <div className={`workspace-grid${canCreate ? "" : " single-column"}`}>
        <section className="panel documents-panel">
          <div className="panel-heading">
            <div>
              <span className="eyebrow">Project portfolio</span>
              <h2>Your projects</h2>
            </div>
            <span className="count">{projects.data?.length ?? 0} projects</span>
          </div>
          {projects.isLoading && <p className="muted">Loading projects…</p>}
          {projects.error && <p className="error" role="alert">{projects.error.message}</p>}
          <div className="resource-list">
            {projects.data?.map((project) => (
              <article key={project.id}>
                <span className="document-icon">P</span>
                <div>
                  <strong>{project.name}</strong>
                  <small>{project.client_name} · {project.description}</small>
                </div>
                <div className="member-actions">
                  <Link to={`/projects/${project.id}`}>View</Link>
                  {canEdit && <Link className="edit" to={`/projects/${project.id}/edit`}>Edit</Link>}
                </div>
              </article>
            ))}
          </div>
          {projects.data?.length === 0 && (
            <div className="empty-state">
              No projects exist in this tenant yet.
            </div>
          )}
        </section>
        {canCreate && (
          <form className="panel create-form" onSubmit={submit}>
            <div className="create-form-heading">
              <span className="create-icon">+</span>
              <div>
                <span className="eyebrow">New project</span>
                <h2>Create project</h2>
              </div>
            </div>
            <label htmlFor="project-client">Client</label>
            <div className="project-client-control">
              <select id="project-client" value={clientId} onChange={(event) => setClientId(event.target.value)} required>
                <option value="">Select a client</option>
                {clients.data?.map((client) => (
                  <option value={client.id} key={client.id}>{client.name}</option>
                ))}
              </select>
              {clients.error && <p className="error" role="alert">{clients.error.message}</p>}
              {clients.data?.length === 0 && <p className="hint">Create a client before adding a project.</p>}
            </div>
            <label htmlFor="project-name">Name</label>
            <input id="project-name" value={name} onChange={(event) => setName(event.target.value)} required />
            <label htmlFor="project-description">Description</label>
            <textarea id="project-description" value={description} onChange={(event) => setDescription(event.target.value)} required />
            {create.error && <p className="error" role="alert">{create.error.message}</p>}
            <button className="primary" disabled={create.isPending || clients.isLoading || clients.data?.length === 0}>
              {create.isPending ? "Creating…" : "Create project"}
            </button>
          </form>
        )}
      </div>
    </section>
  );
}
