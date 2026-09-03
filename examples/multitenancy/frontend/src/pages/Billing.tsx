import { useOutletContext } from "react-router";
import { useInvoices } from "../api/invoices";
import type { SelectedWorkspace } from "../auth/session";
import type { WorkspaceOutletContext } from "../auth/workspace-context";
import { NoWorkspace } from "./Dashboard";

const currency = new Intl.NumberFormat("en-US", {
  style: "currency",
  currency: "USD",
});

export function Billing() {
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
  return <InvoiceHistory workspace={context.selected} />;
}

function InvoiceHistory({ workspace }: { workspace: SelectedWorkspace }) {
  const invoices = useInvoices(workspace);
  return (
    <section className="console-page">
      <header className="console-heading">
        <div>
          <span className="eyebrow">Billing history</span>
          <h1>Invoices</h1>
          <p>
            Invoices are generated automatically when an add-on is purchased
            for {workspace.tenantName}.
          </p>
        </div>
      </header>
      <section className="panel documents-panel invoice-history-panel">
        <div className="panel-heading">
          <div>
            <span className="eyebrow">Purchases</span>
            <h2>Invoice history</h2>
          </div>
          <span className="count">{invoices.data?.length ?? 0} invoices</span>
        </div>
        {invoices.isLoading && <p className="muted">Loading invoices…</p>}
        {invoices.error && (
          <p className="error" role="alert">{invoices.error.message}</p>
        )}
        <div className="invoice-list">
          {invoices.data?.map((invoice) => (
            <div key={invoice.id}>
              <span className="invoice-details">
                <strong className="invoice-number">{invoice.number}</strong>
                <small>{invoice.description}</small>
              </span>
              <strong>{currency.format(invoice.amount_cents / 100)}</strong>
              <span className={`invoice-status ${invoice.status}`}>
                {invoice.status}
              </span>
            </div>
          ))}
        </div>
        {invoices.data?.length === 0 && (
          <div className="empty-state">
            No purchases have generated an invoice yet.
          </div>
        )}
      </section>
    </section>
  );
}
