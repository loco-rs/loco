import { Link, useOutletContext } from "react-router";
import { useDashboard } from "../api/dashboard";
import type { SelectedWorkspace } from "../auth/session";
import type { WorkspaceOutletContext } from "../auth/workspace-context";
import { NoWorkspace } from "./Dashboard";

export function Applications() {
  const context = useOutletContext<WorkspaceOutletContext>();
  if (context.isLoading) return <div className="panel page-state">Loading your workspace…</div>;
  if (context.error) return <p className="error" role="alert">{context.error.message}</p>;
  if (!context.selected) return <NoWorkspace onCreate={context.openWorkspaceCreator} />;
  return <ApplicationList workspace={context.selected} selectApplication={context.selectApplication} />;
}

function ApplicationList({ workspace, selectApplication }: { workspace: SelectedWorkspace; selectApplication: (name: string) => void }) {
  const dashboard = useDashboard(workspace.tenantId);
  if (dashboard.isLoading) return <div className="panel page-state">Loading applications…</div>;
  if (dashboard.error) return <p className="error" role="alert">{dashboard.error.message}</p>;

  return (
    <section className="console-page">
      <header className="console-heading"><div><span className="eyebrow">Workspace services</span><h1>Applications</h1><p>Active products and your effective permissions in each one.</p></div></header>
      <div className="application-grid">
        {dashboard.data?.applications.map((application) => {
          const path = application.name === "Billing" ? "/billing" : "/documents";
          return (
            <article className="panel application-card" key={application.id}>
              <div className="application-card-heading"><span className={`application-icon ${application.name.toLowerCase()}`}>{application.name.charAt(0)}</span><span className="active-badge">{application.status}</span></div>
              <h2>{application.name}</h2>
              <p>{application.name === "Billing" ? "Track tenant invoices and payment status." : "Create and organize tenant-owned records."}</p>
              <div className="permission-list">{application.permissions.map((permission) => <span key={permission}>{permission}</span>)}</div>
              <Link className="application-link" to={path} onClick={() => selectApplication(application.name)}>Open application →</Link>
            </article>
          );
        })}
      </div>
    </section>
  );
}
