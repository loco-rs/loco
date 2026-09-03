import { useEffect, useState, type FormEvent } from "react";
import { Link, useOutletContext, useParams } from "react-router";
import {
  useDashboard,
  useUpdateMemberRole,
  useUpdateRolePermissions,
} from "../api/dashboard";
import type { WorkspaceOutletContext } from "../auth/workspace-context";
import type { MemberAccess } from "../bindings/MemberAccess";
import type { PermissionAccess } from "../bindings/PermissionAccess";
import type { RoleAccess } from "../bindings/RoleAccess";
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
      roles={dashboard.data?.roles ?? []}
      availablePermissions={dashboard.data?.available_permissions ?? []}
    />
  );
}

function MemberProfile({
  canEdit,
  edit,
  member,
  tenantId,
  roles,
  availablePermissions,
}: {
  canEdit: boolean;
  edit: boolean;
  member: MemberAccess;
  tenantId: number;
  roles: RoleAccess[];
  availablePermissions: PermissionAccess[];
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
            <MemberRoleForm
              availablePermissions={availablePermissions}
              member={member}
              roles={roles}
              tenantId={tenantId}
            />
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
  availablePermissions,
  member,
  roles,
  tenantId,
}: {
  availablePermissions: PermissionAccess[];
  member: MemberAccess;
  roles: RoleAccess[];
  tenantId: number;
}) {
  const [role, setRole] = useState(member.roles[0] ?? "Support");
  const selectedRole = roles.find((candidate) => candidate.name === role);
  const selectedRolePermissionIds =
    selectedRole?.permissions.map((permission) => permission.id) ?? [];
  const permissionSignature = selectedRolePermissionIds.join(":");
  const [permissionIds, setPermissionIds] = useState<number[]>(
    selectedRolePermissionIds,
  );
  const updateRole = useUpdateMemberRole(tenantId, member.member_id);
  const updatePermissions = useUpdateRolePermissions(
    tenantId,
    selectedRole?.id ?? 0,
  );

  useEffect(() => {
    setPermissionIds(selectedRolePermissionIds);
  }, [permissionSignature, selectedRole?.id]);

  function submitRole(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    updateRole.mutate({ role });
  }

  function submitPermissions(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (selectedRole) {
      updatePermissions.mutate({ permission_ids: permissionIds });
    }
  }

  function togglePermission(permissionId: number) {
    setPermissionIds((current) =>
      current.includes(permissionId)
        ? current.filter((id) => id !== permissionId)
        : [...current, permissionId],
    );
  }

  return (
    <div className="member-access-forms">
      <form className="member-role-form" onSubmit={submitRole}>
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
            {roles.map((workspaceRole) => (
              <option value={workspaceRole.name} key={workspaceRole.id}>
                {workspaceRole.name}
              </option>
            ))}
          </select>
        </div>
        {updateRole.error && (
          <p className="error" role="alert">{updateRole.error.message}</p>
        )}
        {updateRole.isSuccess && (
          <p className="success" role="status">Member role updated.</p>
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

      <form className="member-permission-form" onSubmit={submitPermissions}>
        <div>
          <span className="eyebrow">Role permissions</span>
          <h2>Permissions for {selectedRole?.name ?? "role"}</h2>
          <p>
            These grants apply to every workspace member assigned the
            {` ${selectedRole?.name ?? "selected"}`} role.
          </p>
        </div>
        <div className="permission-options">
          {availablePermissions.map((permission) => (
            <label className="permission-option" key={permission.id}>
              <input
                type="checkbox"
                checked={permissionIds.includes(permission.id)}
                onChange={() => togglePermission(permission.id)}
              />
              <span>
                <strong>{permission.key}</strong>
                <small>{permission.application_name}</small>
              </span>
            </label>
          ))}
        </div>
        {updatePermissions.error && (
          <p className="error" role="alert">
            {updatePermissions.error.message}
          </p>
        )}
        {updatePermissions.isSuccess && (
          <p className="success" role="status">Role permissions updated.</p>
        )}
        <button
          className="primary permission-submit"
          type="submit"
          disabled={!selectedRole || updatePermissions.isPending}
        >
          {updatePermissions.isPending ? "Saving…" : "Save permissions"}
        </button>
      </form>
    </div>
  );
}
