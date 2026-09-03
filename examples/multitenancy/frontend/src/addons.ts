import type { DashboardAddon } from "./bindings/DashboardAddon";

export function addonsFrom(
  addons: DashboardAddon[] | undefined,
): DashboardAddon[] {
  return [...(addons ?? [])].sort((left, right) => left.name.localeCompare(right.name));
}

export function paidAddonsFrom(
  addons: DashboardAddon[] | undefined,
): DashboardAddon[] {
  return addonsFrom(addons).filter((addon) => addon.status === "active");
}
