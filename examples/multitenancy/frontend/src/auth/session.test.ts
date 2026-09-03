import { beforeEach, describe, expect, it } from "vitest";
import {
  clearSession,
  clearWorkspace,
  getToken,
  isSelectedWorkspace,
  loadWorkspace,
  saveWorkspace,
  setToken,
  type SelectedWorkspace,
} from "./session";

const workspace: SelectedWorkspace = {
  tenantId: 1,
  tenantName: "Acme",
};

describe("authenticated tenant session", () => {
  beforeEach(() => window.localStorage.clear());

  it("stores and clears the token and selected workspace", () => {
    expect(getToken()).toBeNull();
    expect(loadWorkspace()).toBeNull();

    setToken("jwt");
    saveWorkspace(workspace);
    expect(getToken()).toBe("jwt");
    expect(loadWorkspace()).toEqual(workspace);

    clearWorkspace();
    expect(loadWorkspace()).toBeNull();
    saveWorkspace(workspace);
    clearSession();
    expect(getToken()).toBeNull();
    expect(loadWorkspace()).toBeNull();
  });

  it("rejects malformed stored workspace JSON", () => {
    window.localStorage.setItem("loco_multitenancy_workspace", "{");
    expect(loadWorkspace()).toBeNull();
  });

  it("rejects a stored workspace with an invalid shape", () => {
    window.localStorage.setItem("loco_multitenancy_workspace", "{}");
    expect(loadWorkspace()).toBeNull();
  });

  it.each([
    null,
    "workspace",
    {},
    { ...workspace, tenantId: "1" },
    { ...workspace, tenantId: 1.5 },
    { ...workspace, tenantId: 0 },
    { ...workspace, tenantName: 1 },
  ])("rejects invalid workspace %#", (value) => {
    expect(isSelectedWorkspace(value)).toBe(false);
  });
});
