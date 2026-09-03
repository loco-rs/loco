import { describe, expect, it } from "vitest";
import type { PermissionAccess } from "../bindings/PermissionAccess";
import { hasPermission } from "./permissions";

const permissions: PermissionAccess[] = [
  {
    id: 1,
    key: "documents:view",
  },
];

describe("permissions", () => {
  it("finds an effective permission", () => {
    expect(hasPermission(permissions, "documents:view")).toBe(true);
  });

  it("rejects a missing permission", () => {
    expect(hasPermission(permissions, "billing:view")).toBe(false);
  });

  it("supports permissions that have not loaded", () => {
    expect(hasPermission(undefined, "documents:view")).toBe(false);
  });
});
