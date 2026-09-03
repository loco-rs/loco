import { describe, expect, it } from "vitest";
import { addonsFrom } from "./addons";
import type { DashboardApplication } from "./bindings/DashboardApplication";

const applications: DashboardApplication[] = [
  { id: 1, name: "Documents", status: "active", permissions: [] },
  { id: 2, name: "Billing", status: "active", permissions: [] },
  {
    id: 3,
    name: "Analytics",
    status: "inactive",
    permissions: ["analytics:read"],
  },
];

describe("add-ons", () => {
  it("excludes core workspace applications", () => {
    expect(addonsFrom(applications)).toEqual([applications[2]]);
  });

  it("supports dashboards without application data", () => {
    expect(addonsFrom(undefined)).toEqual([]);
  });
});
