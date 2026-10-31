import { afterEach, describe, expect, it, vi } from "vitest";

import { ApiClientError, apiFetch } from "./api";

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("apiFetch", () => {
  it("携带会话 Cookie 并映射 Problem Details", async () => {
    const fetchMock = vi.fn(async () =>
      new Response(
        JSON.stringify({
          type: "https://example.com/errors/auth-session-expired",
          title: "未认证",
          status: 401,
          detail: "会话已失效",
          instance: "/api/v1/me",
          code: "AUTH_SESSION_EXPIRED",
          errors: [],
        }),
        {
          status: 401,
          headers: { "content-type": "application/problem+json" },
        },
      ),
    );
    vi.stubGlobal("fetch", fetchMock);

    await expect(apiFetch("/api/v1/me")).rejects.toMatchObject({
      name: "ApiClientError",
      status: 401,
      code: "AUTH_SESSION_EXPIRED",
      message: "会话已失效",
    });
    expect(fetchMock).toHaveBeenCalledWith(
      "/api/v1/me",
      expect.objectContaining({ credentials: "include" }),
    );
  });

  it("204 返回 undefined，并保留调用方 JSON 头", async () => {
    const fetchMock = vi.fn(async (_path: string, init: RequestInit) => {
      expect(new Headers(init.headers).get("content-type")).toBe("application/json");
      return new Response(null, { status: 204 });
    });
    vi.stubGlobal("fetch", fetchMock);

    await expect(
      apiFetch<void>("/api/v1/students/x", {
        method: "DELETE",
        body: JSON.stringify({ name: "小明" }),
      }),
    ).resolves.toBeUndefined();
  });

  it("无 Problem Details 时退化为 HTTP 错误码", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn(async () => new Response("oops", { status: 502, statusText: "Bad Gateway" })),
    );

    const error = await apiFetch("/api/v1/me").catch((value: unknown) => value);
    expect(error).toBeInstanceOf(ApiClientError);
    expect(error).toMatchObject({ status: 502, code: "HTTP_502" });
  });
});
