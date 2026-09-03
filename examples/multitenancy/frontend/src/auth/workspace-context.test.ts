import { describe, expect, it } from "vitest";
import type { Workspace } from "../bindings/Workspace";
import type { SelectedWorkspace } from "./session";
import {
  flattenWorkspaces,
  resolveWorkspace,
  workspaceSelection,
} from "./workspace-context";

const workspace: Workspace = {
  tenant_id: 1,
  tenant_name: "Acme",
  tenant_slug: "acme",
};

const selected: SelectedWorkspace = {
  tenantId: 1,
  tenantName: "Acme",
};

describe("workspace navigation", () => {
  it("maps tenant workspaces for the navbar", () => {
    expect(flattenWorkspaces(undefined)).toEqual([]);
    expect(flattenWorkspaces([workspace])).toEqual([selected]);
  });

  it("restores a valid selection and falls back to the first workspace", () => {
    const fallback = { ...selected, tenantId: 3 };
    expect(resolveWorkspace([fallback, selected], selected)).toEqual(selected);
    expect(resolveWorkspace([fallback], selected)).toEqual(fallback);
    expect(resolveWorkspace([], null)).toBeUndefined();
  });

  it("selects a created tenant workspace", () => {
    expect(workspaceSelection(workspace)).toEqual(selected);
  });
});
