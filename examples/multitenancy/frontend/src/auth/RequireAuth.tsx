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
