import { describe, expect, it } from "vitest";
import type { PermissionAccess } from "../bindings/PermissionAccess";
import { hasPermission } from "./permissions";

const permissions: PermissionAccess[] = [
  {
    id: 1,
    application_id: 1,
    application_name: "Documents",
    key: "documents:read",
  },
];

describe("permissions", () => {
  it("finds an effective permission", () => {
    expect(hasPermission(permissions, "documents:read")).toBe(true);
  });

  it("rejects a missing permission", () => {
    expect(hasPermission(permissions, "billing:read")).toBe(false);
  });

  it("supports permissions that have not loaded", () => {
    expect(hasPermission(undefined, "documents:read")).toBe(false);
  });
});
