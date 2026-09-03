import { Link, useOutletContext } from "react-router";
import { useDashboard } from "../api/dashboard";
import type { SelectedWorkspace } from "../auth/session";
import type { WorkspaceOutletContext } from "../auth/workspace-context";
import { NoWorkspace } from "./Dashboard";

export function Members() {
  const context = useOutletContext<WorkspaceOutletContext>();
  if (context.isLoading) return <div className="panel page-state">Loading your workspace…</div>;
  if (context.error) return <p className="error" role="alert">{context.error.message}</p>;
  if (!context.selected) return <NoWorkspace onCreate={context.openWorkspaceCreator} />;
  return <MemberList workspace={context.selected} />;
}

function MemberList({ workspace }: { workspace: SelectedWorkspace }) {
  const dashboard = useDashboard(workspace.tenantId);
  if (dashboard.isLoading) return <div className="panel page-state">Loading staff…</div>;
  if (dashboard.error) return <p className="error" role="alert">{dashboard.error.message}</p>;

  const currentMember = dashboard.data?.current_member;
  const canEdit = currentMember?.roles.includes("Owner") ?? false;
  const managedMembers =
    dashboard.data?.members.filter(
      (member) => !member.roles.includes("Owner"),
    ) ?? [];

  return (
    <section className="console-page">
      <header className="console-heading">
        <div><span className="eyebrow">Access directory</span><h1>Staff</h1><p>Roles and effective tenant permissions for this workspace.</p></div>
      </header>
      <div className="panel table-panel">
        <div className="member-table" role="table">
          <div className="member-row member-header" role="row">
            <span>Staff member</span><span>Role</span><span>Permissions</span><span>Actions</span>
          </div>
          {managedMembers.map((member) => (
            <div className="member-row" role="row" key={member.member_id}>
              <div className="member-identity"><span className="avatar">{member.name.charAt(0)}</span><div><strong>{member.name}</strong><small>{member.email}</small></div></div>
              <div>{member.roles.map((role) => <span className="role-badge" key={role}>{role}</span>)}</div>
              <div className="permission-list">{member.permissions.map((permission) => <span key={permission.key}>{permission.key}</span>)}</div>
              <div className="member-actions">
                <Link to={`/staff/${member.member_id}`}>View</Link>
                {canEdit && (
                  <Link
                    className="edit"
                    to={`/staff/${member.member_id}/edit`}
                  >
                    Edit
                  </Link>
                )}
              </div>
            </div>
          ))}
          {managedMembers.length === 0 && (
            <div className="member-table-empty">
              No non-owner staff belong to this workspace yet.
            </div>
          )}
        </div>
      </div>
    </section>
  );
}
