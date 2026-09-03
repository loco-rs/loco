import type { DashboardApplication } from "./bindings/DashboardApplication";

const CORE_APPLICATIONS = new Set(["Documents", "Billing"]);

export function addonsFrom(
  applications: DashboardApplication[] | undefined,
): DashboardApplication[] {
  return (applications ?? []).filter(
    (application) => !CORE_APPLICATIONS.has(application.name),
  );
}
