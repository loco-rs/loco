import { useState, type FormEvent } from "react";
import { useOutletContext } from "react-router";
import { useDashboard } from "../api/dashboard";
import { useCreateInvoice, useInvoices } from "../api/invoices";
import type { SelectedWorkspace } from "../auth/session";
import type { WorkspaceOutletContext } from "../auth/workspace-context";
import { NoWorkspace } from "./Dashboard";

const currency = new Intl.NumberFormat("en-US", { style: "currency", currency: "USD" });

export function Billing() {
  const context = useOutletContext<WorkspaceOutletContext>();
  if (context.isLoading) return <div className="panel page-state">Loading your workspace…</div>;
  if (context.error) return <p className="error" role="alert">{context.error.message}</p>;
  if (!context.selected) return <NoWorkspace onCreate={context.openWorkspaceCreator} />;
  const billing = context.options.find((option) => option.tenantId === context.selected?.tenantId && option.applicationName === "Billing");
  if (!billing) return <div className="panel empty-state">Billing is not active for this workspace.</div>;
  return <BillingWorkspace workspace={billing} />;
}

function BillingWorkspace({ workspace }: { workspace: SelectedWorkspace }) {
  const invoices = useInvoices(workspace);
  const createInvoice = useCreateInvoice(workspace);
  const dashboard = useDashboard(workspace.tenantId);
  const [number, setNumber] = useState("");
  const [amount, setAmount] = useState("");
  const canManage = dashboard.data?.current_member.permissions.some(
    (permission) =>
      permission.application_id === workspace.applicationId &&
      permission.key === "billing:manage",
  );

  function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const amountCents = Math.round(Number(amount) * 100);
    if (!number.trim() || amountCents < 1) return;
    createInvoice.mutate({ number: number.trim(), amount_cents: amountCents, status: "draft" }, { onSuccess: () => { setNumber(""); setAmount(""); } });
  }

  if (dashboard.isLoading) return <div className="panel page-state">Loading billing access…</div>;
  if (dashboard.error) return <p className="error" role="alert">{dashboard.error.message}</p>;

  return (
    <section className="console-page">
      <header className="console-heading"><div><span className="eyebrow">Tenant application</span><h1>Billing</h1><p>Invoices are scoped to {workspace.tenantName} and its Billing subscription.</p></div></header>
      <div className={`workspace-grid${canManage ? "" : " single-column"}`}>
        <section className="panel documents-panel">
          <div className="panel-heading"><div><span className="eyebrow">Accounts receivable</span><h2>Invoices</h2></div><span className="count">{invoices.data?.length ?? 0} invoices</span></div>
          {invoices.isLoading && <p className="muted">Loading invoices…</p>}
          {invoices.error && <p className="error" role="alert">{invoices.error.message}</p>}
          <div className="invoice-list">
            {invoices.data?.map((invoice) => <div key={invoice.id}><span className="invoice-number">{invoice.number}</span><strong>{currency.format(invoice.amount_cents / 100)}</strong><span className={`invoice-status ${invoice.status}`}>{invoice.status}</span></div>)}
          </div>
        </section>
        {canManage ? <form className="panel create-form" onSubmit={submit}>
          <div className="create-form-heading"><span className="create-icon">+</span><div><span className="eyebrow">New charge</span><h2>Create invoice</h2></div></div>
          <label htmlFor="invoice-number">Invoice number</label><input id="invoice-number" value={number} onChange={(event) => setNumber(event.target.value)} placeholder="INV-1003" required />
          <label htmlFor="invoice-amount">Amount (USD)</label><input id="invoice-amount" type="number" min="0.01" step="0.01" value={amount} onChange={(event) => setAmount(event.target.value)} placeholder="125.00" required />
          <button className="primary" type="submit" disabled={createInvoice.isPending}>{createInvoice.isPending ? "Creating…" : "Create draft invoice"}</button>
          {createInvoice.error && <p className="error" role="alert">{createInvoice.error.message}</p>}
        </form> : (
          <aside className="panel access-note">
            <span className="eyebrow">Read-only access</span>
            <h2>Billing access</h2>
            <p>Your current role can review invoices but cannot create them.</p>
          </aside>
        )}
      </div>
    </section>
  );
}
