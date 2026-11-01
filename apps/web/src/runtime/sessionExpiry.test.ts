import { afterEach, describe, expect, it, vi } from "vitest";

import { ApiClientError, apiFetch, setUnauthorizedHandler } from "./api";
import { createQueryClient } from "./queryClient";
import { installSessionExpiryHandler } from "./sessionExpiry";

afterEach(() => {
  setUnauthorizedHandler(null);
  vi.unstubAllGlobals();
});

describe("session expiry", () => {
  it("清空查询缓存并跳转登录", async () => {
    const queryClient = createQueryClient();
    queryClient.setQueryData(["me"], { userId: "u1" });
    const navigate = vi.fn();
    installSessionExpiryHandler(queryClient, { navigate });

    vi.stubGlobal(
      "fetch",
      vi.fn(async () =>
        new Response(
          JSON.stringify({ code: "AUTH_SESSION_EXPIRED", detail: "会话已失效" }),
          { status: 401, headers: { "content-type": "application/problem+json" } },
        ),
      ),
    );

    await expect(apiFetch("/api/v1/students")).rejects.toBeInstanceOf(ApiClientError);
    expect(queryClient.getQueryData(["me"])).toBeUndefined();
    expect(navigate).toHaveBeenCalledWith({ to: "/login", replace: true });
  });
});
