import { Navigate, Outlet } from "react-router";
import { getToken } from "./session";

export function RequireAuth() {
  if (getToken() === null) {
    return <Navigate to="/login" replace />;
  }

  return <Outlet />;
}
