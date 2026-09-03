import { describe, expect, it } from "vitest";
import { addonsFrom } from "./addons";
import type { DashboardAddon } from "./bindings/DashboardAddon";

const addons: DashboardAddon[] = [
  { id: 3, name: "Priority Support", status: "active" },
  { id: 1, name: "Analytics", status: "inactive" },
];

describe("add-ons", () => {
  it("sorts subscription add-ons by name", () => {
    expect(addonsFrom(addons)).toEqual([addons[1], addons[0]]);
  });

  it("supports dashboards without add-on data", () => {
    expect(addonsFrom(undefined)).toEqual([]);
  });
});
