import { Link, Outlet } from "react-router";
import { loadAccess } from "./auth/access";

export function App() {
  const access = loadAccess();

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
          {access && <Link to="/documents">Documents</Link>}
          <Link to="/access">Access context</Link>
        </nav>
      </header>
      <main>
        <Outlet />
      </main>
      <footer>
        Explicit tenant scope · application-aware RBAC · powered by Loco
      </footer>
    </div>
  );
}
