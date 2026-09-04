import { useState, type FormEvent } from "react";
import { Link, useNavigate, useOutletContext } from "react-router";
import { useCreateStaff, useDashboard } from "../api/dashboard";
import type { WorkspaceOutletContext } from "../auth/workspace-context";
import { NoWorkspace } from "./Dashboard";

export function StaffCreate() {
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

  return <StaffCreatePage tenantId={context.selected.tenantId} />;
}

function StaffCreatePage({ tenantId }: { tenantId: number }) {
  const navigate = useNavigate();
  const dashboard = useDashboard(tenantId);
  const createStaff = useCreateStaff(tenantId);
  const [name, setName] = useState("");
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [role, setRole] = useState("Administrator");

  if (dashboard.isLoading) {
    return <div className="panel page-state">Loading staff access…</div>;
  }
  if (dashboard.error) {
    return <p className="error" role="alert">{dashboard.error.message}</p>;
  }

  const canCreate = dashboard.data?.current_member.roles.includes("Owner") ?? false;
  const roles = dashboard.data?.roles.filter((item) => item.name !== "Owner") ?? [];

  function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    createStaff.mutate(
      {
        name: name.trim(),
        email: email.trim(),
        password,
        role,
      },
      {
        onSuccess: (staff) => navigate(`/staff/${staff.member_id}`),
      },
    );
  }

  if (!canCreate) {
    return (
      <section className="panel page-state member-not-found">
        <h2>Owner access required</h2>
        <p>Only the workspace Owner can create staff accounts.</p>
        <Link className="secondary member-page-link" to="/staff">Back to staff</Link>
      </section>
    );
  }

  return (
    <section className="console-page staff-create-page">
      <header className="console-heading">
        <div>
          <span className="eyebrow">Staff management</span>
          <h1>Add staff</h1>
          <p>Create an account and assign its initial workspace role.</p>
        </div>
        <Link className="secondary member-page-link" to="/staff">← Back to staff</Link>
      </header>
      <form className="panel staff-create-form" onSubmit={submit}>
        <div className="staff-form-grid">
          <div>
            <label htmlFor="staff-name">Name</label>
            <input id="staff-name" minLength={2} maxLength={100} value={name} onChange={(event) => setName(event.target.value)} required />
          </div>
          <div>
            <label htmlFor="staff-email">Email</label>
            <input id="staff-email" type="email" value={email} onChange={(event) => setEmail(event.target.value)} required />
          </div>
          <div>
            <label htmlFor="staff-password">Temporary password</label>
            <input id="staff-password" type="password" minLength={8} maxLength={128} value={password} onChange={(event) => setPassword(event.target.value)} required />
          </div>
          <div>
            <label htmlFor="staff-role">Role</label>
            <select id="staff-role" value={role} onChange={(event) => setRole(event.target.value)} required>
              {roles.map((item) => <option value={item.name} key={item.id}>{item.name}</option>)}
            </select>
          </div>
        </div>
        <p className="hint">The staff member can sign in immediately with this email and password.</p>
        {createStaff.error && <p className="error" role="alert">{createStaff.error.message}</p>}
        <div className="button-row staff-create-actions">
          <Link className="secondary member-page-link" to="/staff">Cancel</Link>
          <button className="primary" type="submit" disabled={createStaff.isPending}>
            {createStaff.isPending ? "Creating…" : "Create staff member"}
          </button>
        </div>
      </form>
    </section>
  );
}
