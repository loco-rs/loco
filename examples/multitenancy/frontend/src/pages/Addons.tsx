import { Link, useOutletContext } from "react-router";
import { addonDetailsFor } from "../addon-catalog";
import { usePurchaseAddon } from "../api/addons";
import { useDashboard } from "../api/dashboard";
import { addonsFrom } from "../addons";
import { hasPermission } from "../auth/permissions";
import type { SelectedWorkspace } from "../auth/session";
import type { WorkspaceOutletContext } from "../auth/workspace-context";
import { AddonIcon } from "../components/AddonIcon";
import { NoWorkspace } from "./Dashboard";

const ADDON_PRICES: Record<string, string> = {
  Analytics: "$49",
  "Approval Workflows": "$29",
  "Feature Flags": "$39",
  "Priority Support": "$99",
};

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
  const purchase = usePurchaseAddon(workspace);
  if (dashboard.isLoading) {
    return <div className="panel page-state">Loading add-ons…</div>;
  }
  if (dashboard.error) {
    return <p className="error" role="alert">{dashboard.error.message}</p>;
  }

  const addons = addonsFrom(dashboard.data?.addons);
  const canPurchase = hasPermission(
    dashboard.data?.current_member.permissions,
    "billing:purchase",
  );

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
          const details = addonDetailsFor(addon.name);
          return (
            <article
              className="panel application-card"
              id={`addon-${addon.id}`}
              key={addon.id}
            >
              <div className="application-card-heading">
                <AddonIcon name={addon.name} />
                <span className={`active-badge ${addon.status}`}>
                  {active ? "Included" : "Not included"}
                </span>
              </div>
              <h2>{addon.name}</h2>
              <p>{details.description}</p>
              <span className="application-availability">
                {active
                  ? "Available through this workspace subscription"
                  : "Available to purchase for this workspace"}
              </span>
              {!active && canPurchase && (
                <button
                  className="primary addon-purchase"
                  type="button"
                  disabled={purchase.isPending}
                  onClick={() => purchase.mutate(addon.id)}
                >
                  {purchase.isPending && purchase.variables === addon.id
                    ? "Completing purchase…"
                    : `Purchase for ${ADDON_PRICES[addon.name] ?? "$19"}`}
                </button>
              )}
              {active && (
                <Link
                  className="secondary addon-open-link"
                  to={`/addons/${addon.id}`}
                >
                  Open add-on
                </Link>
              )}
              {!active && !canPurchase && (
                <span className="addon-purchase-note">
                  Ask an Owner or Administrator to purchase this add-on.
                </span>
              )}
              {purchase.error && purchase.variables === addon.id && (
                <p className="error" role="alert">{purchase.error.message}</p>
              )}
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
