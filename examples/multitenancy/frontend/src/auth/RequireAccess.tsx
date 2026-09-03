import { Navigate, Outlet } from "react-router";
import { hasAccess } from "./access";

export function RequireAccess() {
  if (!hasAccess()) {
    return <Navigate to="/access" replace />;
  }

  return <Outlet />;
}
