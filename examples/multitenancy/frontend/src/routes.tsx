import { Navigate, createBrowserRouter } from "react-router";
import { App } from "./App";
import { RequireAuth } from "./auth/RequireAuth";
import { getToken } from "./auth/session";
import { Documents } from "./pages/Documents";
import { Applications } from "./pages/Applications";
import { Billing } from "./pages/Billing";
import { Dashboard } from "./pages/Dashboard";
import { Login } from "./pages/Login";
import { Members } from "./pages/Members";
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
          { path: "documents", element: <Documents /> },
          { path: "billing", element: <Billing /> },
          { path: "members", element: <Members /> },
          { path: "applications", element: <Applications /> },
        ],
      },
    ],
  },
]);
