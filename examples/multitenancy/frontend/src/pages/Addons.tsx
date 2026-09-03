import { useOutletContext } from "react-router";
import { useDashboard } from "../api/dashboard";
import { addonsFrom } from "../addons";
import type { SelectedWorkspace } from "../auth/session";
import type { WorkspaceOutletContext } from "../auth/workspace-context";
import { NoWorkspace } from "./Dashboard";

export function Addons() {
  const context = useOutletContext<WorkspaceOutletContext>();
  if (context.isLoading) {
    return <div className="panel page-state">Loading your workspace…</div>;
  }
  if (context.error) {
    return <p className="error" role="alert">{context.error.message}</p>;
  }
  if (!context.selected) {
    return <NoWorkspace onCreate={context.openWorkspaceCreator} />;
  }
  return <AddonList workspace={context.selected} />;
}

function AddonList({ workspace }: { workspace: SelectedWorkspace }) {
  const dashboard = useDashboard(workspace.tenantId);
  if (dashboard.isLoading) {
    return <div className="panel page-state">Loading add-ons…</div>;
  }
  if (dashboard.error) {
    return <p className="error" role="alert">{dashboard.error.message}</p>;
  }

  const addons = addonsFrom(dashboard.data?.applications);

  return (
    <section className="console-page">
      <header className="console-heading">
        <div>
          <span className="eyebrow">Subscription catalog</span>
          <h1>Add-ons</h1>
          <p>
            Optional workspace features are available according to your
            purchased subscription.
          </p>
        </div>
      </header>
      <div className="application-grid addon-grid">
        {addons.map((addon) => {
          const active = addon.status === "active";
          return (
            <article className="panel application-card" key={addon.id}>
              <div className="application-card-heading">
                <span className={`application-icon ${addon.name.toLowerCase()}`}>
                  {addon.name.charAt(0)}
                </span>
                <span className={`active-badge ${addon.status}`}>
                  {active ? "Included" : "Not included"}
                </span>
              </div>
              <h2>{addon.name}</h2>
              <p>Explore workspace activity and usage trends.</p>
              {active && addon.permissions.length > 0 && (
                <div className="permission-list">
                  {addon.permissions.map((permission) => (
                    <span key={permission}>{permission}</span>
                  ))}
                </div>
              )}
              <span className="application-availability">
                {active
                  ? "Available through this workspace subscription"
                  : "Available to purchase for this workspace"}
              </span>
            </article>
          );
        })}
        {addons.length === 0 && (
          <div className="panel empty-state">No add-ons are available yet.</div>
        )}
      </div>
    </section>
  );
}
