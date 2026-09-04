import { Link, useOutletContext, useParams } from "react-router";
import { addonDetailsFor } from "../addon-catalog";
import { useDashboard } from "../api/dashboard";
import type { WorkspaceOutletContext } from "../auth/workspace-context";
import { AddonIcon } from "../components/AddonIcon";
import { NoWorkspace } from "./Dashboard";

export function AddonDetails() {
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

  return <SubscribedAddon tenantId={context.selected.tenantId} />;
}

function SubscribedAddon({ tenantId }: { tenantId: number }) {
  const dashboard = useDashboard(tenantId);
  const addonId = Number(useParams().addonId);

  if (dashboard.isLoading) {
    return <div className="panel page-state">Loading add-on…</div>;
  }
  if (dashboard.error) {
    return <p className="error" role="alert">{dashboard.error.message}</p>;
  }

  const addon = dashboard.data?.addons.find((item) => item.id === addonId);
  if (!addon) {
    return (
      <section className="panel page-state addon-unavailable">
        <h2>Add-on not found</h2>
        <p>This add-on is not available for the selected workspace.</p>
        <Link className="secondary member-page-link" to="/addons">
          Back to add-ons
        </Link>
      </section>
    );
  }
  if (addon.status !== "active") {
    return (
      <section className="panel page-state addon-unavailable">
        <AddonIcon name={addon.name} />
        <h2>{addon.name} is not included</h2>
        <p>Purchase this add-on before opening its workspace page.</p>
        <Link className="primary member-page-link" to={`/addons#addon-${addon.id}`}>
          View add-on
        </Link>
      </section>
    );
  }

  const details = addonDetailsFor(addon.name);
  return (
    <section className="console-page addon-detail-page">
      <header className="console-heading addon-detail-heading">
        <div>
          <span className="eyebrow">Subscribed add-on</span>
          <h1>{addon.name}</h1>
          <p>{details.description}</p>
        </div>
        <Link className="secondary member-page-link" to="/addons">
          ← Back to add-ons
        </Link>
      </header>
      <section className="panel addon-detail-hero">
        <AddonIcon name={addon.name} />
        <div>
          <span className="active-badge active">Active</span>
          <h2>Ready for this workspace</h2>
          <p>{details.introduction}</p>
        </div>
      </section>
      <div className="addon-highlight-grid">
        {details.highlights.map((highlight) => (
          <article className="panel addon-highlight-card" key={highlight.title}>
            <span className="addon-highlight-check" aria-hidden="true">
              ✓
            </span>
            <h3>{highlight.title}</h3>
            <p>{highlight.description}</p>
          </article>
        ))}
      </div>
      <p className="addon-demo-note">
        This is a demonstration page. Product functionality can be connected
        here later.
      </p>
    </section>
  );
}
