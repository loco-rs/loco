import { useState, type FormEvent } from "react";
import { useOutletContext } from "react-router";
import { useDashboard, useUpdateMemberRole } from "../api/dashboard";
import type { SelectedWorkspace } from "../auth/session";
import type { WorkspaceOutletContext } from "../auth/workspace-context";
import type { MemberAccess } from "../bindings/MemberAccess";
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
  const [viewing, setViewing] = useState<MemberAccess | null>(null);
  const [editing, setEditing] = useState<MemberAccess | null>(null);
  if (dashboard.isLoading) return <div className="panel page-state">Loading members…</div>;
  if (dashboard.error) return <p className="error" role="alert">{dashboard.error.message}</p>;

  const currentMember = dashboard.data?.current_member;
  const canEdit = currentMember?.roles.includes("Owner") ?? false;

  return (
    <section className="console-page">
      <header className="console-heading">
        <div><span className="eyebrow">Access directory</span><h1>Members</h1><p>Roles and effective application permissions for this workspace.</p></div>
      </header>
      <div className="panel table-panel">
        <div className="member-table" role="table">
          <div className="member-row member-header" role="row">
            <span>Member</span><span>Role</span><span>Permissions</span><span>Actions</span>
          </div>
          {dashboard.data?.members.map((member) => (
            <div className="member-row" role="row" key={member.member_id}>
              <div className="member-identity"><span className="avatar">{member.name.charAt(0)}</span><div><strong>{member.name}</strong><small>{member.email}</small></div></div>
              <div>{member.roles.map((role) => <span className="role-badge" key={role}>{role}</span>)}</div>
              <div className="permission-list">{member.permissions.map((permission) => <span key={`${permission.application_id}:${permission.key}`}>{permission.key}</span>)}</div>
              <div className="member-actions">
                <button type="button" onClick={() => setViewing(member)}>View</button>
                {canEdit && (
                  <button
                    className="edit"
                    type="button"
                    disabled={member.user_id === currentMember?.user_id}
                    title={member.user_id === currentMember?.user_id ? "You cannot change your own role" : "Edit member role"}
                    onClick={() => setEditing(member)}
                  >
                    Edit
                  </button>
                )}
              </div>
            </div>
          ))}
        </div>
      </div>
      {viewing && <MemberDetails member={viewing} onClose={() => setViewing(null)} />}
      {editing && <EditMemberRole tenantId={workspace.tenantId} member={editing} onClose={() => setEditing(null)} />}
    </section>
  );
}

function MemberDetails({ member, onClose }: { member: MemberAccess; onClose: () => void }) {
  return (
    <div className="workspace-modal-backdrop" role="presentation" onMouseDown={onClose}>
      <section className="panel member-modal" role="dialog" aria-modal="true" aria-labelledby="member-details-title" onMouseDown={(event) => event.stopPropagation()}>
        <button className="modal-close" type="button" aria-label="Close" onClick={onClose}>×</button>
        <span className="eyebrow">Member access</span>
        <h2 id="member-details-title">{member.name}</h2>
        <p className="member-modal-email">{member.email}</p>
        <div className="member-detail-section"><strong>Role</strong><div>{member.roles.map((role) => <span className="role-badge" key={role}>{role}</span>)}</div></div>
        <div className="member-detail-section"><strong>Effective permissions</strong><div className="permission-list">{member.permissions.map((permission) => <span key={`${permission.application_id}:${permission.key}`}>{permission.key}</span>)}</div></div>
      </section>
    </div>
  );
}

function EditMemberRole({ tenantId, member, onClose }: { tenantId: number; member: MemberAccess; onClose: () => void }) {
  const [role, setRole] = useState(member.roles[0] ?? "Viewer");
  const updateRole = useUpdateMemberRole(tenantId, member.member_id);

  function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    updateRole.mutate({ role }, { onSuccess: onClose });
  }

  return (
    <div className="workspace-modal-backdrop" role="presentation" onMouseDown={onClose}>
      <section className="panel member-modal" role="dialog" aria-modal="true" aria-labelledby="edit-member-title" onMouseDown={(event) => event.stopPropagation()}>
        <button className="modal-close" type="button" aria-label="Close" onClick={onClose}>×</button>
        <span className="eyebrow">Workspace role</span>
        <h2 id="edit-member-title">Edit {member.name}</h2>
        <p className="member-modal-email">Choose the member’s role and effective permission set.</p>
        <form onSubmit={submit}>
          <label htmlFor="member-role">Role</label>
          <select id="member-role" value={role} onChange={(event) => setRole(event.target.value)}>
            <option value="Owner">Owner</option>
            <option value="Manager">Manager</option>
            <option value="Viewer">Viewer</option>
          </select>
          {updateRole.error && <p className="error" role="alert">{updateRole.error.message}</p>}
          <div className="modal-actions">
            <button className="secondary" type="button" onClick={onClose}>Cancel</button>
            <button className="primary" type="submit" disabled={updateRole.isPending}>{updateRole.isPending ? "Saving…" : "Save role"}</button>
          </div>
        </form>
      </section>
    </div>
  );
}
