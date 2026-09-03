import { useQueryClient } from "@tanstack/react-query";
import { Link, Outlet, useNavigate } from "react-router";
import { clearSession, getToken } from "./auth/session";

export function App() {
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const authenticated = getToken() !== null;

  function logout() {
    clearSession();
    queryClient.clear();
    navigate("/login");
  }

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
        <nav>
          {authenticated ? (
            <>
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
        <Outlet />
      </main>
      <footer>
        Tenant-aware sessions · application RBAC · powered by Loco
      </footer>
    </div>
  );
}
