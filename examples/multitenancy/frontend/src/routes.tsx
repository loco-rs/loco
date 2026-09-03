import { Navigate, createBrowserRouter } from "react-router";
import { App } from "./App";
import { RequireAuth } from "./auth/RequireAuth";
import { getToken } from "./auth/session";
import { Documents } from "./pages/Documents";
import { Login } from "./pages/Login";
import { Register } from "./pages/Register";

export const router = createBrowserRouter([
  {
    path: "/",
    element: <App />,
    children: [
      {
        index: true,
        element: <Navigate to={getToken() ? "/documents" : "/login"} replace />,
      },
      { path: "login", element: <Login /> },
      { path: "register", element: <Register /> },
      {
        element: <RequireAuth />,
        children: [{ path: "documents", element: <Documents /> }],
      },
    ],
  },
]);
