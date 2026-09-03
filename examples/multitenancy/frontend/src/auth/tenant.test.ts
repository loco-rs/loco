import { describe, expect, it } from "vitest";
import { tenantSlug } from "./tenant";

describe("tenant slug", () => {
  it("derives a lowercase slug from the tenant name", () => {
    expect(tenantSlug("  Acme Research & Labs  ")).toBe("acme-research-labs");
  });

  it("removes unsupported leading and trailing characters", () => {
    expect(tenantSlug("---Loco.rs---")).toBe("loco-rs");
  });
});
