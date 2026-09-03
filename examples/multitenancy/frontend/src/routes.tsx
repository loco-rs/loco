import { Navigate, createBrowserRouter } from "react-router";
import { App } from "./App";
import { RequireAccess } from "./auth/RequireAccess";
import { hasAccess } from "./auth/access";
import { Access } from "./pages/Access";
import { Documents } from "./pages/Documents";

export const router = createBrowserRouter([
  {
    path: "/",
    element: <App />,
    children: [
      {
        index: true,
        element: <Navigate to={hasAccess() ? "/documents" : "/access"} replace />,
      },
      { path: "access", element: <Access /> },
      {
        element: <RequireAccess />,
        children: [{ path: "documents", element: <Documents /> }],
      },
    ],
  },
]);
