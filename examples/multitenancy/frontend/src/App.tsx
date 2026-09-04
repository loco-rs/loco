import { useQueryClient } from "@tanstack/react-query";
import { useEffect, useRef, useState } from "react";
import { Link, NavLink, Outlet, useLocation, useNavigate } from "react-router";
import { useCurrentUser, useWorkspaces } from "./api/auth";
import { useDashboard } from "./api/dashboard";
import { paidAddonsFrom } from "./addons";
import { hasPermission } from "./auth/permissions";
import {
  clearSession,
  getToken,
  loadWorkspace,
  saveWorkspace,
} from "./auth/session";
import {
  flattenWorkspaces,
  resolveWorkspace,
  workspaceSelection,
  type WorkspaceOutletContext,
} from "./auth/workspace-context";
import type { Workspace } from "./bindings/Workspace";
import { AddonIcon } from "./components/AddonIcon";
import { WorkspaceCreator } from "./components/WorkspaceCreator";

export function App() {
  const location = useLocation();
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const authenticated = getToken() !== null;
  const workspaces = useWorkspaces(authenticated);
  const currentUser = useCurrentUser(authenticated);
  const workspaceMenuRef = useRef<HTMLDetailsElement>(null);
  const userMenuRef = useRef<HTMLDetailsElement>(null);
  const [savedWorkspace, setSavedWorkspace] = useState(loadWorkspace);
  const [workspaceCreatorOpen, setWorkspaceCreatorOpen] = useState(false);
  const promptWorkspaceCreation =
    typeof location.state === "object" &&
    location.state !== null &&
    "createWorkspace" in location.state &&
    location.state.createWorkspace === true;

  const workspaceOptions = flattenWorkspaces(workspaces.data);
  const selectedWorkspace = resolveWorkspace(workspaceOptions, savedWorkspace);
  const selectedTenant = workspaces.data?.find(
    (workspace) => workspace.tenant_id === selectedWorkspace?.tenantId,
  );
  const dashboard = useDashboard(
    authenticated ? selectedWorkspace?.tenantId : undefined,
  );
  const currentPermissions = dashboard.data?.current_member.permissions;
  const canViewClients = hasPermission(currentPermissions, "clients:view");
  const canViewProjects = hasPermission(currentPermissions, "projects:view");
  const canViewDocuments = hasPermission(currentPermissions, "documents:view");
  const canViewBilling = hasPermission(currentPermissions, "billing:view");
  const paidAddons = paidAddonsFrom(dashboard.data?.addons);

  useEffect(() => {
    if (promptWorkspaceCreation) {
      setWorkspaceCreatorOpen(true);
      void navigate(location.pathname, { replace: true, state: null });
    }
  }, [location.pathname, navigate, promptWorkspaceCreation]);

  useEffect(() => {
    function closeMenus(event: PointerEvent) {
      if (!(event.target instanceof Node)) {
        return;
      }

      for (const menu of [workspaceMenuRef.current, userMenuRef.current]) {
        if (menu?.open && !menu.contains(event.target)) {
          menu.removeAttribute("open");
        }
      }
    }

    document.addEventListener("pointerdown", closeMenus);
    return () => document.removeEventListener("pointerdown", closeMenus);
  }, []);

  function chooseWorkspace(workspace: Workspace) {
    const next = workspaceSelection(workspace);
    saveWorkspace(next);
    setSavedWorkspace(next);
    workspaceMenuRef.current?.removeAttribute("open");
  }

  function createWorkspaceFromMenu() {
    workspaceMenuRef.current?.removeAttribute("open");
    setWorkspaceCreatorOpen(true);
  }

  function activateWorkspace(workspace: Workspace) {
    const next = workspaceSelection(workspace);
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
    options: workspaceOptions,
    isLoading: workspaces.isLoading,
    error: workspaces.error,
    openWorkspaceCreator: () => setWorkspaceCreatorOpen(true),
  };
  const userInitials =
    currentUser.data?.name
      .split(/\s+/)
      .map((part) => part.charAt(0))
      .join("")
      .slice(0, 2)
      .toUpperCase() || "U";

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
              <details className="workspace-menu" ref={workspaceMenuRef}>
                <summary aria-label="Open workspace menu">
                  <WorkspaceIcon />
                  <span>{selectedWorkspace?.tenantName ?? "Choose workspace"}</span>
                  <span className="menu-chevron" aria-hidden="true">
                    <svg viewBox="0 0 24 24">
                      <path d="m19.5 8.25-7.5 7.5-7.5-7.5" />
                    </svg>
                  </span>
                </summary>
                <div className="workspace-menu-popover">
                  <div className="workspace-menu-heading">
                    <span className="workspace-menu-icon"><WorkspaceIcon /></span>
                    <div>
                      <strong>{selectedTenant?.tenant_name ?? "Your workspaces"}</strong>
                      <span>
                        {selectedTenant
                          ? `Tenant workspace · /${selectedTenant.tenant_slug}`
                          : "Select a workspace"}
                      </span>
                    </div>
                  </div>
                  <div className="workspace-menu-options">
                    <span className="workspace-menu-label">Switch workspace</span>
                    {workspaces.data?.map((workspace) => {
                      const active = workspace.tenant_id === selectedWorkspace?.tenantId;
                      return (
                        <button
                          className={active ? "active" : undefined}
                          type="button"
                          key={workspace.tenant_id}
                          onClick={() => chooseWorkspace(workspace)}
                        >
                          <span className="workspace-app-icon workspace">
                            {workspace.tenant_name.charAt(0)}
                          </span>
                          <span>
                            <strong>{workspace.tenant_name}</strong>
                            <small>/{workspace.tenant_slug}</small>
                          </span>
                          {active && <span className="workspace-check" aria-label="Current">✓</span>}
                        </button>
                      );
                    })}
                  </div>
                  <button className="workspace-menu-create" type="button" onClick={createWorkspaceFromMenu}>
                    <span aria-hidden="true">＋</span>
                    New workspace
                  </button>
                </div>
              </details>
              <details className="user-menu" ref={userMenuRef}>
                <summary aria-label="Open account menu">
                  <span className="user-avatar" aria-hidden="true">
                    {userInitials}
                  </span>
                </summary>
                <div className="user-menu-popover">
                  <div className="workspace-menu-heading user-menu-heading">
                    <span className="user-menu-avatar" aria-hidden="true">
                      {userInitials}
                    </span>
                    <div>
                      <strong>{currentUser.data?.name ?? "Account"}</strong>
                      <span>{currentUser.data?.email ?? "Loading…"}</span>
                    </div>
                  </div>
                  <button
                    className="workspace-menu-create user-menu-sign-out"
                    type="button"
                    onClick={logout}
                  >
                    <SignOutIcon />
                    <span>Log out</span>
                  </button>
                </div>
              </details>
            </>
          ) : (
            <>
              <Link to="/login">Login</Link>
              <Link to="/register">Register</Link>
            </>
          )}
        </nav>
      </header>
      <main className={authenticated ? "authenticated-main" : undefined}>
        {authenticated ? (
          <div className="dashboard-layout">
            <aside className="sidebar">
              <nav className="sidebar-nav" aria-label="Workspace navigation">
                <div className="sidebar-group overview-group">
                  <NavLink to="/dashboard">
                    <SidebarIcon name="overview" />
                    <span>Overview</span>
                  </NavLink>
                </div>
                <div className="sidebar-group">
                  <span className="sidebar-label">Core</span>
                  {canViewClients && (
                    <NavLink to="/clients">
                      <SidebarIcon name="clients" />
                      <span>Clients</span>
                    </NavLink>
                  )}
                  {canViewProjects && (
                    <NavLink to="/projects">
                      <SidebarIcon name="projects" />
                      <span>Projects</span>
                    </NavLink>
                  )}
                  {canViewDocuments && (
                    <NavLink to="/documents">
                      <SidebarIcon name="documents" />
                      <span>Documents</span>
                    </NavLink>
                  )}
                </div>
                <div className="sidebar-group">
                  <span className="sidebar-label">Settings</span>
                  <NavLink to="/staff">
                    <SidebarIcon name="staff" />
                    <span>Staff</span>
                  </NavLink>
                </div>
                <div className="sidebar-group">
                  <span className="sidebar-label">Billing</span>
                  {canViewBilling && (
                    <NavLink to="/billing">
                      <SidebarIcon name="invoices" />
                      <span>Invoices</span>
                    </NavLink>
                  )}
                  <NavLink to="/addons" end>
                    <SidebarIcon name="addons" />
                    <span>Add-ons</span>
                  </NavLink>
                </div>
                {paidAddons.length > 0 && (
                  <div className="sidebar-group paid-addon-group">
                    <span className="sidebar-label">Paid</span>
                    {paidAddons.map((addon) => (
                      <NavLink to={`/addons/${addon.id}`} key={addon.id}>
                        <AddonIcon name={addon.name} variant="sidebar" />
                        <span>{addon.name}</span>
                      </NavLink>
                    ))}
                  </div>
                )}
              </nav>
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
        Tenant-aware sessions · workspace RBAC · powered by Loco
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

function WorkspaceIcon() {
  return (
    <svg className="workspace-building-icon" viewBox="0 0 24 24" aria-hidden="true">
      <path d="M3 21h18M5 21V7l7-3v17M12 10h7v11M8 9h1M8 13h1M8 17h1M15 13h1M15 17h1" />
    </svg>
  );
}

function SignOutIcon() {
  return (
    <svg className="sign-out-icon" viewBox="0 0 24 24" aria-hidden="true">
      <path d="M15.75 9V5.25A2.25 2.25 0 0 0 13.5 3h-6A2.25 2.25 0 0 0 5.25 5.25v13.5A2.25 2.25 0 0 0 7.5 21h6a2.25 2.25 0 0 0 2.25-2.25V15M18 15l3-3m0 0-3-3m3 3H9" />
    </svg>
  );
}

type SidebarIconName =
  | "overview"
  | "clients"
  | "projects"
  | "documents"
  | "staff"
  | "invoices"
  | "addons";

const SIDEBAR_ICON_PATHS: Record<SidebarIconName, string[]> = {
  overview: ["M4 4h6v6H4zM14 4h6v6h-6zM4 14h6v6H4zM14 14h6v6h-6z"],
  clients: [
    "M16 20v-1.5a4.5 4.5 0 0 0-4.5-4.5h-3A4.5 4.5 0 0 0 4 18.5V20",
    "M10 10a3 3 0 1 0 0-6 3 3 0 0 0 0 6M17 8a2.5 2.5 0 1 0 0-5M18 13a4 4 0 0 1 3 3.87V20",
  ],
  projects: ["M3 7.5h7l2-2h9v14H3z"],
  documents: ["M6 3h8l4 4v14H6zM14 3v5h4M9 12h6M9 16h6"],
  staff: [
    "M16 20v-1.5a4.5 4.5 0 0 0-4.5-4.5h-3A4.5 4.5 0 0 0 4 18.5V20",
    "M10 10a3 3 0 1 0 0-6 3 3 0 0 0 0 6M17 10a3 3 0 1 0 0-6M18 14a4 4 0 0 1 3 3.87V20",
  ],
  invoices: ["M6 3h12v18l-2.5-1.5L13 21l-2.5-1.5L8 21l-2-1.5zM9 8h6M9 12h6M9 16h3"],
  addons: ["M8.5 3v3M15.5 3v3M8.5 18v3M15.5 18v3M3 8.5h3M18 8.5h3M3 15.5h3M18 15.5h3M7 7h10v10H7z"],
};

function SidebarIcon({ name }: { name: SidebarIconName }) {
  return (
    <svg className="sidebar-icon" viewBox="0 0 24 24" aria-hidden="true">
      {SIDEBAR_ICON_PATHS[name].map((path) => (
        <path d={path} key={path} />
      ))}
    </svg>
  );
}
