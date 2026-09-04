import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act } from "react";
import { createRoot } from "react-dom/client";
import {
  Outlet,
  RouterProvider,
  createMemoryRouter,
} from "react-router";
import { describe, expect, it } from "vitest";
import { dashboardKeys } from "../api/dashboard";
import type { WorkspaceOutletContext } from "../auth/workspace-context";
import type { DashboardDto } from "../bindings/DashboardDto";
import { AddonDetails } from "./AddonDetails";

const workspaceContext: WorkspaceOutletContext = {
  selected: { tenantId: 1, tenantName: "Designer" },
  options: [],
  isLoading: false,
  error: null,
  openWorkspaceCreator: () => undefined,
};

async function renderAddon(status: string, addonId = 1) {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { staleTime: Infinity } },
  });
  queryClient.setQueryData(dashboardKeys.detail(1), {
    addons: [{ id: 1, name: "Analytics", status }],
  } as DashboardDto);
  const router = createMemoryRouter(
    [
      {
        element: <Outlet context={workspaceContext} />,
        children: [
          { path: "/addons/:addonId", element: <AddonDetails /> },
        ],
      },
    ],
    { initialEntries: [`/addons/${addonId}`] },
  );
  const container = document.createElement("div");
  const root = createRoot(container);

  await act(async () => {
    root.render(
      <QueryClientProvider client={queryClient}>
        <RouterProvider router={router} />
      </QueryClientProvider>,
    );
  });

  return { container, root };
}

describe("AddonDetails", () => {
  it("shows the demonstration page for a subscribed add-on", async () => {
    const { container, root } = await renderAddon("active");

    expect(container.textContent).toContain("Analytics");
    expect(container.textContent).toContain("Ready for this workspace");
    expect(container.textContent).toContain("Activity snapshot");
    await act(async () => root.unmount());
  });

  it("does not open an add-on that is not subscribed", async () => {
    const { container, root } = await renderAddon("inactive");

    expect(container.textContent).toContain("Analytics is not included");
    expect(container.textContent).toContain("Purchase this add-on");
    await act(async () => root.unmount());
  });

  it("handles an unknown add-on route", async () => {
    const { container, root } = await renderAddon("active", 99);

    expect(container.textContent).toContain("Add-on not found");
    await act(async () => root.unmount());
  });
});
