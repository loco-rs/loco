import { useState, type FormEvent } from "react";
import { Link, useNavigate, useOutletContext, useParams } from "react-router";
import { useDashboard, useUpdateMemberRole } from "../api/dashboard";
import type { WorkspaceOutletContext } from "../auth/workspace-context";
import type { MemberAccess } from "../bindings/MemberAccess";
import { NoWorkspace } from "./Dashboard";

export function MemberManagement({ edit = false }: { edit?: boolean }) {
  const context = useOutletContext<WorkspaceOutletContext>();
  const { memberId: memberIdParam } = useParams();
  const memberId = Number(memberIdParam);

  if (context.isLoading) {
    return <div className="panel page-state">Loading your workspace…</div>;
  }
  if (context.error) {
    return <p className="error" role="alert">{context.error.message}</p>;
  }
  if (!context.selected) {
    return <NoWorkspace onCreate={context.openWorkspaceCreator} />;
  }

  return (
    <MemberManagementPage
      edit={edit}
      memberId={memberId}
      tenantId={context.selected.tenantId}
    />
  );
}

function MemberManagementPage({
  edit,
  memberId,
  tenantId,
}: {
  edit: boolean;
  memberId: number;
  tenantId: number;
}) {
  const dashboard = useDashboard(tenantId);

  if (dashboard.isLoading) {
    return <div className="panel page-state">Loading member…</div>;
  }
  if (dashboard.error) {
    return <p className="error" role="alert">{dashboard.error.message}</p>;
  }

  const member = dashboard.data?.members.find(
    (candidate) => candidate.member_id === memberId,
  );
  if (!Number.isInteger(memberId) || !member) {
    return (
      <section className="panel page-state member-not-found">
        <h2>Member not found</h2>
        <p>This member does not belong to the selected workspace.</p>
        <Link className="secondary member-page-link" to="/members">
          Back to members
        </Link>
      </section>
    );
  }

  const currentMember = dashboard.data?.current_member;
  const canEdit =
    currentMember?.roles.includes("Owner") === true &&
    member.user_id !== currentMember.user_id &&
    !member.roles.includes("Owner");

  return (
    <MemberProfile
      canEdit={canEdit}
      edit={edit}
      member={member}
      tenantId={tenantId}
    />
  );
}

function MemberProfile({
  canEdit,
  edit,
  member,
  tenantId,
}: {
  canEdit: boolean;
  edit: boolean;
  member: MemberAccess;
  tenantId: number;
}) {
  return (
    <section className="console-page member-management-page">
      <header className="console-heading member-management-heading">
        <div>
          <span className="eyebrow">Member management</span>
          <h1>{edit ? `Edit ${member.name}` : member.name}</h1>
          <p>{member.email}</p>
        </div>
        <Link className="secondary member-page-link" to="/members">
          ← Back to members
        </Link>
      </header>

      <div className="panel member-management-panel">
        <div className="member-profile-heading">
          <span className="member-profile-avatar" aria-hidden="true">
            {member.name.charAt(0)}
          </span>
          <div>
            <strong>{member.name}</strong>
            <span>{member.email}</span>
          </div>
          {!edit && canEdit && (
            <Link
              className="primary member-page-link"
              to={`/members/${member.member_id}/edit`}
            >
              Edit role
            </Link>
          )}
        </div>

        {edit ? (
          canEdit ? (
            <MemberRoleForm member={member} tenantId={tenantId} />
          ) : (
            <div className="member-management-message">
              <strong>This role cannot be edited</strong>
              <p>Only workspace owners can update non-owner member roles.</p>
            </div>
          )
        ) : (
          <div className="member-management-grid">
            <div className="member-detail-section">
              <strong>Workspace role</strong>
              <div>
                {member.roles.map((role) => (
                  <span className="role-badge" key={role}>{role}</span>
                ))}
              </div>
            </div>
            <div className="member-detail-section">
              <strong>Effective permissions</strong>
              <div className="permission-list">
                {member.permissions.map((permission) => (
                  <span key={`${permission.application_id}:${permission.key}`}>
                    {permission.key}
                  </span>
                ))}
              </div>
            </div>
          </div>
        )}
      </div>
    </section>
  );
}

function MemberRoleForm({
  member,
  tenantId,
}: {
  member: MemberAccess;
  tenantId: number;
}) {
  const navigate = useNavigate();
  const [role, setRole] = useState(member.roles[0] ?? "Viewer");
  const updateRole = useUpdateMemberRole(tenantId, member.member_id);

  function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    updateRole.mutate(
      { role },
      {
        onSuccess: () => {
          void navigate(`/members/${member.member_id}`, { replace: true });
        },
      },
    );
  }

  return (
    <form className="member-role-form" onSubmit={submit}>
      <div>
        <span className="eyebrow">Workspace role</span>
        <h2>Change member access</h2>
        <p>Select the role that controls this member’s effective permissions.</p>
      </div>
      <div className="member-role-field">
        <label htmlFor="member-role">Role</label>
        <select
          id="member-role"
          value={role}
          onChange={(event) => setRole(event.target.value)}
        >
          <option value="Owner">Owner</option>
          <option value="Manager">Manager</option>
          <option value="Viewer">Viewer</option>
        </select>
      </div>
      {updateRole.error && (
        <p className="error" role="alert">{updateRole.error.message}</p>
      )}
      <div className="button-row member-role-actions">
        <Link
          className="secondary member-page-link"
          to={`/members/${member.member_id}`}
        >
          Cancel
        </Link>
        <button className="primary" type="submit" disabled={updateRole.isPending}>
          {updateRole.isPending ? "Saving…" : "Save role"}
        </button>
      </div>
    </form>
  );
}
