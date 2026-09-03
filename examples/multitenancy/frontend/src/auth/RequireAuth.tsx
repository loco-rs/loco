import { Navigate, Outlet, useOutletContext } from "react-router";
import { getToken } from "./session";
import type { WorkspaceOutletContext } from "./workspace-context";

export function RequireAuth() {
  const workspaceContext = useOutletContext<WorkspaceOutletContext>();

  if (getToken() === null) {
    return <Navigate to="/login" replace />;
  }

  return <Outlet context={workspaceContext} />;
}

export function PublicOnly() {
  if (getToken() !== null) {
    return <Navigate to="/dashboard" replace />;
  }

  return (
    <div className="public-shell">
      <main>
        <Outlet />
      </main>
    </div>
  );
}
