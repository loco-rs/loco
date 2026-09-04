import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { AddonIcon, addonClassName } from "./AddonIcon";

const ADDON_NAMES = [
  "Analytics",
  "Approval Workflows",
  "Feature Flags",
  "Priority Support",
];

describe("AddonIcon", () => {
  it("renders a distinct icon for every catalog add-on", () => {
    const icons = ADDON_NAMES.map((name) =>
      renderToStaticMarkup(<AddonIcon name={name} />),
    );

    expect(new Set(icons).size).toBe(ADDON_NAMES.length);
  });

  it("renders the compact sidebar variant", () => {
    const icon = renderToStaticMarkup(
      <AddonIcon name="Analytics" variant="sidebar" />,
    );

    expect(icon).toContain("addon-sidebar-icon");
    expect(icon).not.toContain("application-icon");
  });

  it("builds the add-on color class", () => {
    expect(addonClassName("Approval Workflows")).toBe("approval-workflows");
  });
});
