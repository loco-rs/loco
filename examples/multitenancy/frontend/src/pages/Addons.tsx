import { useOutletContext } from "react-router";
import { usePurchaseAddon } from "../api/addons";
import { useDashboard } from "../api/dashboard";
import { addonsFrom } from "../addons";
import { hasPermission } from "../auth/permissions";
import type { SelectedWorkspace } from "../auth/session";
import type { WorkspaceOutletContext } from "../auth/workspace-context";
import { NoWorkspace } from "./Dashboard";

const ADDON_DESCRIPTIONS: Record<string, string> = {
  Analytics: "Explore workspace activity and usage trends.",
  "Client Portal": "Share work and collect feedback from clients.",
  "Feature Flags": "Control staged feature releases across environments.",
  "Priority Support": "Get expedited help from the support team.",
};

const ADDON_PRICES: Record<string, string> = {
  Analytics: "$49",
  "Client Portal": "$29",
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
          const iconClass = addon.name.toLowerCase().replace(/ /g, "-");
          return (
            <article className="panel application-card" key={addon.id}>
              <div className="application-card-heading">
                <span className={`application-icon ${iconClass}`}>
                  {addon.name.charAt(0)}
                </span>
                <span className={`active-badge ${addon.status}`}>
                  {active ? "Included" : "Not included"}
                </span>
              </div>
              <h2>{addon.name}</h2>
              <p>
                {ADDON_DESCRIPTIONS[addon.name] ??
                  "Extend this workspace with an optional feature."}
              </p>
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
