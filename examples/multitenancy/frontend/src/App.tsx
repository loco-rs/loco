import { useQueryClient } from "@tanstack/react-query";
import { useEffect, useState } from "react";
import { Link, NavLink, Outlet, useLocation, useNavigate } from "react-router";
import { useCurrentUser, useWorkspaces } from "./api/auth";
import {
  clearSession,
  getToken,
  loadWorkspace,
  saveWorkspace,
} from "./auth/session";
import {
  CREATE_WORKSPACE_VALUE,
  flattenWorkspaces,
  resolveWorkspace,
  workspaceSelection,
  type WorkspaceOutletContext,
} from "./auth/workspace-context";
import type { Workspace } from "./bindings/Workspace";
import { WorkspaceCreator } from "./components/WorkspaceCreator";

export function App() {
  const location = useLocation();
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const authenticated = getToken() !== null;
  const workspaces = useWorkspaces(authenticated);
  const currentUser = useCurrentUser(authenticated);
  const [savedWorkspace, setSavedWorkspace] = useState(loadWorkspace);
  const [workspaceCreatorOpen, setWorkspaceCreatorOpen] = useState(false);
  const promptWorkspaceCreation =
    typeof location.state === "object" &&
    location.state !== null &&
    "createWorkspace" in location.state &&
    location.state.createWorkspace === true;

  const workspaceOptions = flattenWorkspaces(workspaces.data);
  const selectedWorkspace = resolveWorkspace(workspaceOptions, savedWorkspace);

  useEffect(() => {
    if (promptWorkspaceCreation) {
      setWorkspaceCreatorOpen(true);
      void navigate(location.pathname, { replace: true, state: null });
    }
  }, [location.pathname, navigate, promptWorkspaceCreation]);

  function selectWorkspace(value: string) {
    if (value === CREATE_WORKSPACE_VALUE) {
      setWorkspaceCreatorOpen(true);
      return;
    }

    const next = workspaceOptions.find(
      (option) => `${option.tenantId}:${option.applicationId}` === value,
    );
    if (next) {
      saveWorkspace(next);
      setSavedWorkspace(next);
    }
  }

  function activateWorkspace(workspace: Workspace) {
    const next = workspaceSelection(workspace);
    if (!next) {
      return;
    }
    saveWorkspace(next);
    setSavedWorkspace(next);
    setWorkspaceCreatorOpen(false);
  }

  function selectApplication(applicationName: string) {
    const next = workspaceOptions.find(
      (option) =>
        option.tenantId === selectedWorkspace?.tenantId &&
        option.applicationName === applicationName,
    );
    if (next) {
      saveWorkspace(next);
      setSavedWorkspace(next);
    }
  }

  function logout() {
    clearSession();
    queryClient.clear();
    setSavedWorkspace(null);
    setWorkspaceCreatorOpen(false);
    navigate("/login");
  }

  const workspaceContext: WorkspaceOutletContext = {
    selected: selectedWorkspace,
    options: workspaceOptions,
    isLoading: workspaces.isLoading,
    error: workspaces.error,
    openWorkspaceCreator: () => setWorkspaceCreatorOpen(true),
    selectApplication,
  };

  return (
    <div className="shell">
      <header className="topbar">
        <Link className="brand" to="/">
          <span className="brand-mark">L</span>
          <span>
            <strong>Loco</strong>
            <small>Multi-tenancy</small>
          </span>
        </Link>
        <nav className={authenticated ? "authenticated-nav" : undefined}>
          {authenticated ? (
            <>
              <label className="nav-workspace-picker">
                <span className="sr-only">Current workspace</span>
                <select
                  aria-label="Current workspace"
                  value={
                    selectedWorkspace
                      ? `${selectedWorkspace.tenantId}:${selectedWorkspace.applicationId}`
                      : ""
                  }
                  disabled={workspaces.isLoading}
                  onChange={(event) => selectWorkspace(event.target.value)}
                >
                  {!selectedWorkspace && (
                    <option value="" disabled>
                      {workspaces.isLoading ? "Loading workspaces…" : "Choose workspace"}
                    </option>
                  )}
                  {workspaceOptions.map((option) => (
                    <option
                      key={`${option.tenantId}:${option.applicationId}`}
                      value={`${option.tenantId}:${option.applicationId}`}
                    >
                      {option.tenantName} · {option.applicationName}
                    </option>
                  ))}
                  <option value={CREATE_WORKSPACE_VALUE}>＋ New workspace</option>
                </select>
              </label>
              <details className="user-menu">
                <summary aria-label="Open account menu">
                  <span className="user-avatar" aria-hidden="true">
                    {currentUser.data?.name
                      .split(/\s+/)
                      .map((part) => part.charAt(0))
                      .join("")
                      .slice(0, 2)
                      .toUpperCase() || "U"}
                  </span>
                </summary>
                <div className="user-menu-popover">
                  <div className="user-menu-identity">
                    <strong>{currentUser.data?.name ?? "Account"}</strong>
                    <span>{currentUser.data?.email ?? "Loading…"}</span>
                  </div>
                  <button type="button" onClick={logout}>
                    <span className="sign-out-icon" aria-hidden="true">↪</span>
                    Sign out
                  </button>
                </div>
              </details>
            </>
          ) : (
            <>
              <Link to="/login">Log in</Link>
              <Link to="/register">Register</Link>
            </>
          )}
        </nav>
      </header>
      <main className={authenticated ? "authenticated-main" : undefined}>
        {authenticated ? (
          <div className="dashboard-layout">
            <aside className="sidebar">
              <div className="sidebar-context">
                <span className="eyebrow">Workspace</span>
                <strong>{selectedWorkspace?.tenantName ?? "No workspace"}</strong>
                <small>{selectedWorkspace?.applicationName ?? "Create one to begin"}</small>
              </div>
              <nav className="sidebar-nav" aria-label="Workspace navigation">
                <NavLink to="/dashboard">Overview</NavLink>
                <NavLink
                  to="/documents"
                  onClick={() => selectApplication("Documents")}
                >
                  Documents
                </NavLink>
                <NavLink
                  to="/billing"
                  onClick={() => selectApplication("Billing")}
                >
                  Billing
                </NavLink>
                <NavLink to="/members">Members</NavLink>
                <NavLink to="/applications">Applications</NavLink>
              </nav>
              <div className="sidebar-note">
                <span className="status-dot" />
                Tenant scope enforced
              </div>
            </aside>
            <div className="dashboard-content">
              <Outlet context={workspaceContext} />
            </div>
          </div>
        ) : (
          <Outlet context={workspaceContext} />
        )}
      </main>
      <footer>
        Tenant-aware sessions · application RBAC · powered by Loco
      </footer>
      {authenticated && workspaceCreatorOpen && (
        <WorkspaceCreator
          onClose={() => setWorkspaceCreatorOpen(false)}
          onCreated={activateWorkspace}
        />
      )}
    </div>
  );
}
