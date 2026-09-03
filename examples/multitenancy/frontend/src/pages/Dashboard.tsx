import { Link, useOutletContext } from "react-router";
import { useDashboard } from "../api/dashboard";
import { addonsFrom } from "../addons";
import type { SelectedWorkspace } from "../auth/session";
import type { WorkspaceOutletContext } from "../auth/workspace-context";
import { hasPermission } from "../auth/permissions";

export function Dashboard() {
  const context = useOutletContext<WorkspaceOutletContext>();
  return <WorkspaceDashboard context={context} />;
}

function WorkspaceDashboard({ context }: { context: WorkspaceOutletContext }) {
  if (context.isLoading) {
    return <div className="panel page-state">Loading your workspace…</div>;
  }
  if (context.error) {
    return <p className="error" role="alert">{context.error.message}</p>;
  }
  if (!context.selected) {
    return <NoWorkspace onCreate={context.openWorkspaceCreator} />;
  }
  return <DashboardDetails workspace={context.selected} />;
}

function DashboardDetails({ workspace }: { workspace: SelectedWorkspace }) {
  const dashboard = useDashboard(workspace.tenantId);

  if (dashboard.isLoading) {
    return <div className="panel page-state">Loading workspace overview…</div>;
  }
  if (dashboard.error) {
    return <p className="error" role="alert">{dashboard.error.message}</p>;
  }
  if (!dashboard.data) {
    return null;
  }

  const { stats, current_member: currentMember } = dashboard.data;
  const addons = addonsFrom(dashboard.data.addons);
  const canViewClients = hasPermission(currentMember.permissions, "clients:view");
  const canViewProjects = hasPermission(currentMember.permissions, "projects:view");
  const canViewDocuments = hasPermission(
    currentMember.permissions,
    "documents:view",
  );
  return (
    <section className="console-page">
      <header className="console-heading">
        <div>
          <span className="eyebrow">Workspace overview</span>
          <h1>{dashboard.data.tenant_name}</h1>
          <p>
            People, add-ons, and tenant-scoped records in one place. You
            are signed in as {currentMember.roles.join(" · ")}.
          </p>
        </div>
      </header>

      <div className="stat-grid">
        <Stat label="Staff" value={stats.member_count} />
        {canViewClients && <Stat label="Clients" value={stats.client_count} />}
        {canViewProjects && <Stat label="Projects" value={stats.project_count} />}
        {canViewDocuments && <Stat label="Documents" value={stats.document_count} />}
      </div>

      <div className="overview-grid">
        <section className="panel overview-panel">
          <div className="panel-heading">
            <div><span className="eyebrow">Team access</span><h2>Staff</h2></div>
            <Link to="/staff">View all</Link>
          </div>
          <div className="compact-list">
            {dashboard.data.members.map((member) => (
              <div key={member.member_id}>
                <span className="avatar">{member.name.charAt(0)}</span>
                <div><strong>{member.name}</strong><small>{member.email}</small></div>
                <span className="role-badge">{member.roles.join(", ")}</span>
              </div>
            ))}
          </div>
        </section>

        <section className="panel overview-panel">
          <div className="panel-heading">
            <div><span className="eyebrow">Subscriptions</span><h2>Add-ons</h2></div>
            <Link to="/addons">View all</Link>
          </div>
          <div className="application-list">
            {addons.map((addon) => (
              <div key={addon.id}>
                <span className={`application-icon ${addon.name.toLowerCase().replace(/ /g, "-")}`}>
                  {addon.name.charAt(0)}
                </span>
                <div><strong>{addon.name}</strong><small>{addon.status === "active" ? "Included in subscription" : "Available to purchase"}</small></div>
                <span className={`active-badge ${addon.status}`}>{addon.status}</span>
              </div>
            ))}
          </div>
        </section>
      </div>
    </section>
  );
}

function Stat({ label, value }: { label: string; value: number }) {
  return <article className="panel stat-card"><span>{label}</span><strong>{value}</strong></article>;
}

export function NoWorkspace({ onCreate }: { onCreate: () => void }) {
  return (
    <section className="panel empty-state no-workspace">
      <span className="eyebrow">Get started</span>
      <h1>Create a workspace</h1>
      <p>Your account needs a tenant workspace before its core features can be used.</p>
      <button className="primary" type="button" onClick={onCreate}>New workspace</button>
    </section>
  );
}
