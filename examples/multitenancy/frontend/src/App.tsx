import { useQueryClient } from "@tanstack/react-query";
import { useEffect, useState } from "react";
import { Link, Outlet, useLocation, useNavigate } from "react-router";
import { useWorkspaces } from "./api/auth";
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

  function logout() {
    clearSession();
    queryClient.clear();
    setSavedWorkspace(null);
    setWorkspaceCreatorOpen(false);
    navigate("/login");
  }

  const workspaceContext: WorkspaceOutletContext = {
    selected: selectedWorkspace,
    isLoading: workspaces.isLoading,
    error: workspaces.error,
    openWorkspaceCreator: () => setWorkspaceCreatorOpen(true),
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
              <Link to="/documents">Documents</Link>
              <button className="nav-button" type="button" onClick={logout}>
                Log out
              </button>
            </>
          ) : (
            <>
              <Link to="/login">Log in</Link>
              <Link to="/register">Register</Link>
            </>
          )}
        </nav>
      </header>
      <main>
        <Outlet context={workspaceContext} />
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
