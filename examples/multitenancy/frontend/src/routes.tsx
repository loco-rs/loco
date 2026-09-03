import { Navigate, createBrowserRouter } from "react-router";
import { App } from "./App";
import { RequireAuth } from "./auth/RequireAuth";
import { getToken } from "./auth/session";
import { Documents } from "./pages/Documents";
import { DocumentManagement } from "./pages/DocumentManagement";
import { Addons } from "./pages/Addons";
import { Billing } from "./pages/Billing";
import { ClientManagement } from "./pages/ClientManagement";
import { Clients } from "./pages/Clients";
import { Dashboard } from "./pages/Dashboard";
import { Login } from "./pages/Login";
import { MemberManagement } from "./pages/MemberManagement";
import { Members } from "./pages/Members";
import { ProjectManagement } from "./pages/ProjectManagement";
import { Projects } from "./pages/Projects";
import { Register } from "./pages/Register";

export const router = createBrowserRouter([
  {
    path: "/",
    element: <App />,
    children: [
      {
        index: true,
        element: <Navigate to={getToken() ? "/dashboard" : "/login"} replace />,
      },
      { path: "login", element: <Login /> },
      { path: "register", element: <Register /> },
      {
        element: <RequireAuth />,
        children: [
          { path: "dashboard", element: <Dashboard /> },
          { path: "clients", element: <Clients /> },
          { path: "clients/:clientId", element: <ClientManagement /> },
          { path: "clients/:clientId/edit", element: <ClientManagement edit /> },
          { path: "projects", element: <Projects /> },
          { path: "projects/:projectId", element: <ProjectManagement /> },
          { path: "projects/:projectId/edit", element: <ProjectManagement edit /> },
          { path: "documents", element: <Documents /> },
          { path: "documents/:documentId", element: <DocumentManagement /> },
          { path: "documents/:documentId/edit", element: <DocumentManagement edit /> },
          { path: "billing", element: <Billing /> },
          { path: "members", element: <Members /> },
          { path: "members/:memberId", element: <MemberManagement /> },
          { path: "members/:memberId/edit", element: <MemberManagement edit /> },
          { path: "addons", element: <Addons /> },
          { path: "applications", element: <Navigate to="/addons" replace /> },
        ],
      },
    ],
  },
]);
