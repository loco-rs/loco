import { beforeEach, describe, expect, it, vi } from "vitest";
import { saveAccess } from "../auth/access";
import { ApiClientError, get, post } from "./client";

describe("API client", () => {
  beforeEach(() => {
    window.localStorage.clear();
    vi.restoreAllMocks();
  });

  it("requires an access context", async () => {
    await expect(get("/api/documents")).rejects.toMatchObject({
      status: 0,
      message: "Configure an API key, tenant, and application first",
    });
  });

  it("sends authenticated GET requests", async () => {
    saveAccess({ apiKey: "lo-test", tenantId: 1, applicationId: 1 });
    const response = [{ id: 1, title: "Roadmap" }];
    const fetchMock = vi
      .spyOn(window, "fetch")
      .mockResolvedValue(new Response(JSON.stringify(response), { status: 200 }));

    await expect(get("/api/documents")).resolves.toEqual(response);
    expect(fetchMock).toHaveBeenCalledWith("/api/documents", {
      method: "GET",
      headers: { Authorization: "Bearer lo-test" },
      credentials: "same-origin",
      body: undefined,
    });
  });

  it("serializes POST bodies", async () => {
    saveAccess({ apiKey: "lo-test", tenantId: 1, applicationId: 1 });
    vi.spyOn(window, "fetch").mockResolvedValue(
      new Response(JSON.stringify({ id: 2, title: "Launch" }), {
        status: 200,
      }),
    );

    await post("/api/documents", { title: "Launch" });
    expect(window.fetch).toHaveBeenCalledWith("/api/documents", {
      method: "POST",
      headers: {
        Authorization: "Bearer lo-test",
        "Content-Type": "application/json",
      },
      credentials: "same-origin",
      body: JSON.stringify({ title: "Launch" }),
    });
  });

  it("surfaces structured Loco errors", async () => {
    saveAccess({ apiKey: "lo-test", tenantId: 1, applicationId: 1 });
    vi.spyOn(window, "fetch").mockResolvedValue(
      new Response(
        JSON.stringify({ error: "unauthorized", description: "No permission" }),
        { status: 401 },
      ),
    );

    const error = await get("/api/documents").catch((reason: unknown) => reason);
    expect(error).toBeInstanceOf(ApiClientError);
    expect(error).toMatchObject({ status: 401, message: "No permission" });
  });

  it("falls back when an error body is not JSON", async () => {
    saveAccess({ apiKey: "lo-test", tenantId: 1, applicationId: 1 });
    vi.spyOn(window, "fetch").mockResolvedValue(
      new Response("service unavailable", { status: 503 }),
    );

    await expect(get("/api/documents")).rejects.toMatchObject({
      status: 503,
      message: "Request failed with status 503",
      body: null,
    });
  });

  it("ignores primitive JSON error bodies", async () => {
    saveAccess({ apiKey: "lo-test", tenantId: 1, applicationId: 1 });
    vi.spyOn(window, "fetch").mockResolvedValue(
      new Response(JSON.stringify("error"), { status: 400 }),
    );

    await expect(get("/api/documents")).rejects.toMatchObject({
      status: 400,
      message: "Request failed with status 400",
      body: null,
    });
  });
});
