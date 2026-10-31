import { afterEach, describe, expect, it, vi } from "vitest";
import { isRedirect } from "@tanstack/react-router";

import { createQueryClient } from "./queryClient";
import { requireSession } from "./session";

afterEach(() => {
  vi.unstubAllGlobals();
});

function meResponse(tenantId = "tenant-1") {
  return new Response(
    JSON.stringify({
      userId: "user-1",
      email: "teacher@example.com",
      tenantId,
      tenantName: "租户",
      role: "owner",
      quota: { limit: 15, used: 0, remaining: 15, resetAt: "2026-11-01T00:00:00Z" },
    }),
    { status: 200, headers: { "content-type": "application/json" } },
  );
}

describe("requireSession", () => {
  it("成功时把会话返回给路由", async () => {
    vi.stubGlobal("fetch", vi.fn(async () => meResponse()));
    const queryClient = createQueryClient();

    const me = await requireSession({
      context: { queryClient },
      location: { href: "/students" },
    });

    expect(me.tenantId).toBe("tenant-1");
    expect(vi.mocked(fetch)).toHaveBeenCalledTimes(1);
  });

  it("未登录跳转登录页", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn(
        async () =>
          new Response(JSON.stringify({ code: "AUTH_SESSION_EXPIRED", detail: "会话已失效" }), {
            status: 401,
            headers: { "content-type": "application/problem+json" },
          }),
      ),
    );
    const queryClient = createQueryClient();

    const error = await requireSession({
      context: { queryClient },
      location: { href: "/students" },
    }).catch((value: unknown) => value);

    expect(isRedirect(error)).toBe(true);
    expect(error).toMatchObject({ options: { to: "/login" } });
  });

  it("未选租户跳转租户选择页", async () => {
    vi.stubGlobal("fetch", vi.fn(async () => meResponse("")));
    const queryClient = createQueryClient();

    const error = await requireSession({
      context: { queryClient },
      location: { href: "/students" },
    }).catch((value: unknown) => value);

    expect(isRedirect(error)).toBe(true);
    expect(error).toMatchObject({ options: { to: "/select-tenant" } });
  });

  it("401 后移除失败缓存，登录后重新请求 /me", async () => {
    let calls = 0;
    vi.stubGlobal(
      "fetch",
      vi.fn(async () => {
        calls += 1;
        if (calls === 1) {
          return new Response(
            JSON.stringify({ code: "AUTH_SESSION_EXPIRED", detail: "会话已失效" }),
            { status: 401, headers: { "content-type": "application/problem+json" } },
          );
        }
        return meResponse();
      }),
    );
    const queryClient = createQueryClient();

    await requireSession({
      context: { queryClient },
      location: { href: "/students" },
    }).catch(() => undefined);
    const me = await requireSession({
      context: { queryClient },
      location: { href: "/students" },
    });

    expect(me.tenantId).toBe("tenant-1");
    expect(calls).toBe(2);
  });

  it("同一 QueryClient 复用 /me 请求", async () => {
    vi.stubGlobal("fetch", vi.fn(async () => meResponse()));
    const queryClient = createQueryClient();

    await requireSession({ context: { queryClient }, location: { href: "/" } });
    await requireSession({ context: { queryClient }, location: { href: "/students" } });

    expect(vi.mocked(fetch)).toHaveBeenCalledTimes(1);
  });
});
