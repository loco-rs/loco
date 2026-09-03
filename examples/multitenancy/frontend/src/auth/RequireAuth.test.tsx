import { act } from "react";
import { createRoot } from "react-dom/client";
import {
  Outlet,
  RouterProvider,
  createMemoryRouter,
  useOutletContext,
} from "react-router";
import { afterEach, describe, expect, it } from "vitest";
import { setToken } from "./session";
import { RequireAuth } from "./RequireAuth";
import type { WorkspaceOutletContext } from "./workspace-context";

const workspaceContext: WorkspaceOutletContext = {
  selected: {
    tenantId: 1,
    tenantName: "Acme",
    applicationId: 2,
    applicationName: "Documents",
  },
  options: [],
  isLoading: false,
  error: null,
  openWorkspaceCreator: () => undefined,
  selectApplication: () => undefined,
};

function WorkspaceProbe() {
  const context = useOutletContext<WorkspaceOutletContext>();
  return <span>{context.selected?.tenantName}</span>;
}

function createRouter() {
  return createMemoryRouter(
    [
      {
        element: <Outlet context={workspaceContext} />,
        children: [
          {
            element: <RequireAuth />,
            children: [{ path: "/", element: <WorkspaceProbe /> }],
          },
          { path: "/login", element: <span>Login page</span> },
        ],
      },
    ],
    { initialEntries: ["/"] },
  );
}

afterEach(() => window.localStorage.clear());

describe("authenticated route context", () => {
  it("forwards the parent workspace context", async () => {
    setToken("jwt");
    const container = document.createElement("div");
    const root = createRoot(container);

    await act(async () => root.render(<RouterProvider router={createRouter()} />));
    expect(container.textContent).toBe("Acme");
    await act(async () => root.unmount());
  });

  it("redirects unauthenticated users", async () => {
    const container = document.createElement("div");
    const root = createRoot(container);

    await act(async () => root.render(<RouterProvider router={createRouter()} />));
    expect(container.textContent).toBe("Login page");
    await act(async () => root.unmount());
  });
});
