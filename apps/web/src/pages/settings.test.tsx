// @vitest-environment jsdom
import { QueryClientProvider } from "@tanstack/react-query";
import {
  RouterProvider,
  createMemoryHistory,
  createRootRoute,
  createRoute,
  createRouter,
  Outlet,
} from "@tanstack/react-router";
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import "@testing-library/jest-dom/vitest";
import { afterEach, describe, expect, it, vi } from "vitest";

import { SettingsPage } from "./SettingsPage";
import { createQueryClient } from "../runtime/queryClient";

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});

describe("SettingsPage", () => {
  it("登出后清除会话状态并返回登录页", async () => {
    vi.spyOn(window, "scrollTo").mockImplementation(() => undefined);
    const fetchMock = vi.fn(async (url: string) => {
      if (url === "/api/v1/me") {
        return new Response(
          JSON.stringify({
            userId: "u1",
            email: "teacher@example.com",
            tenantId: "t1",
            tenantName: "租户",
            role: "owner",
            quota: { limit: 15, used: 1, remaining: 14, resetAt: "2026-11-01T00:00:00Z" },
          }),
          { status: 200, headers: { "content-type": "application/json" } },
        );
      }
      if (url === "/auth/logout") {
        return new Response(null, { status: 204 });
      }
      throw new Error(`unexpected ${url}`);
    });
    vi.stubGlobal("fetch", fetchMock);

    const queryClient = createQueryClient();
    const rootRoute = createRootRoute({ component: () => <Outlet /> });
    const settingsRoute = createRoute({
      getParentRoute: () => rootRoute,
      path: "/settings",
      component: SettingsPage,
    });
    const loginRoute = createRoute({
      getParentRoute: () => rootRoute,
      path: "/login",
      component: () => <div>登录页</div>,
    });
    const router = createRouter({
      routeTree: rootRoute.addChildren([settingsRoute, loginRoute]),
      history: createMemoryHistory({ initialEntries: ["/settings"] }),
      context: { queryClient },
    });

    render(
      <QueryClientProvider client={queryClient}>
        <RouterProvider router={router} />
      </QueryClientProvider>,
    );

    expect(await screen.findByText(/teacher@example.com/)).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "退出登录" }));

    expect(await screen.findByText("登录页")).toBeInTheDocument();
    expect(fetchMock).toHaveBeenCalledWith(
      "/auth/logout",
      expect.objectContaining({ credentials: "include", method: "POST" }),
    );
    expect(queryClient.getQueryData(["me"])).toBeUndefined();
  });
});

describe("SettingsPage expired logout", () => {
  it("会话已过期时仍清空本地状态并返回登录页", async () => {
    vi.spyOn(window, "scrollTo").mockImplementation(() => undefined);
    const fetchMock = vi.fn(async (url: string) => {
      if (url === "/api/v1/me") {
        return new Response(
          JSON.stringify({
            userId: "u1",
            email: "teacher@example.com",
            tenantId: "t1",
            tenantName: "租户",
            role: "owner",
            quota: { limit: 15, used: 1, remaining: 14, resetAt: "2026-11-01T00:00:00Z" },
          }),
          { status: 200, headers: { "content-type": "application/json" } },
        );
      }
      if (url === "/auth/logout") {
        return new Response(JSON.stringify({ code: "AUTH_SESSION_EXPIRED", detail: "expired" }), {
          status: 401,
          headers: { "content-type": "application/problem+json" },
        });
      }
      throw new Error(`unexpected ${url}`);
    });
    vi.stubGlobal("fetch", fetchMock);

    const queryClient = createQueryClient();
    const rootRoute = createRootRoute({ component: () => <Outlet /> });
    const settingsRoute = createRoute({
      getParentRoute: () => rootRoute,
      path: "/settings",
      component: SettingsPage,
    });
    const loginRoute = createRoute({
      getParentRoute: () => rootRoute,
      path: "/login",
      component: () => <div>登录页</div>,
    });
    const router = createRouter({
      routeTree: rootRoute.addChildren([settingsRoute, loginRoute]),
      history: createMemoryHistory({ initialEntries: ["/settings"] }),
      context: { queryClient },
    });

    render(
      <QueryClientProvider client={queryClient}>
        <RouterProvider router={router} />
      </QueryClientProvider>,
    );

    await screen.findByText(/teacher@example.com/);
    await userEvent.click(screen.getByRole("button", { name: "退出登录" }));

    expect(await screen.findByText("登录页")).toBeInTheDocument();
    expect(queryClient.getQueryData(["me"])).toBeUndefined();
  });
});
