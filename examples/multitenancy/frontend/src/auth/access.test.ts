import { beforeEach, describe, expect, it } from "vitest";
import {
  clearAccess,
  hasAccess,
  isAccessContext,
  loadAccess,
  saveAccess,
  type AccessContext,
} from "./access";

const access: AccessContext = {
  apiKey: "lo-example",
  tenantId: 1,
  applicationId: 2,
};

describe("tenant access storage", () => {
  beforeEach(() => window.localStorage.clear());

  it("round-trips and clears a valid context", () => {
    expect(loadAccess()).toBeNull();
    expect(hasAccess()).toBe(false);

    saveAccess(access);
    expect(loadAccess()).toEqual(access);
    expect(hasAccess()).toBe(true);

    clearAccess();
    expect(loadAccess()).toBeNull();
  });

  it("rejects malformed JSON", () => {
    window.localStorage.setItem("loco_multitenancy_access", "{");
    expect(loadAccess()).toBeNull();
  });

  it("rejects a stored object with an invalid shape", () => {
    window.localStorage.setItem("loco_multitenancy_access", "{}");
    expect(loadAccess()).toBeNull();
  });

  it.each([
    null,
    "access",
    {},
    { ...access, apiKey: "" },
    { ...access, apiKey: 1 },
    { ...access, tenantId: "1" },
    { ...access, tenantId: 1.5 },
    { ...access, tenantId: 0 },
    { ...access, applicationId: "2" },
    { ...access, applicationId: 2.5 },
    { ...access, applicationId: 0 },
  ])("rejects invalid context %#", (value) => {
    expect(isAccessContext(value)).toBe(false);
  });
});
