// @vitest-environment jsdom
import { QueryClientProvider } from "@tanstack/react-query";
import {
  Outlet,
  RouterProvider,
  createMemoryHistory,
  createRootRoute,
  createRoute,
  createRouter,
} from "@tanstack/react-router";
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import "@testing-library/jest-dom/vitest";
import { afterEach, describe, expect, it, vi } from "vitest";

import { RegisterPage } from "./RegisterPage";
import { createQueryClient } from "../runtime/queryClient";

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});

describe("RegisterPage", () => {
  it("注册成功后刷新会话并进入工作台", async () => {
    vi.spyOn(window, "scrollTo").mockImplementation(() => undefined);
    const fetchMock = vi.fn(async (url: string) => {
      if (url === "/auth/register") {
        return new Response(JSON.stringify({ userId: "u1", tenantId: "t1" }), {
          status: 201,
          headers: { "content-type": "application/json" },
        });
      }
      if (url === "/api/v1/me") {
        return new Response(
          JSON.stringify({
            userId: "u1",
            email: "teacher@example.com",
            tenantId: "t1",
            tenantName: "租户",
            role: "owner",
            quota: { limit: 15, used: 0, remaining: 15, resetAt: "2026-11-01T00:00:00Z" },
          }),
          { status: 200, headers: { "content-type": "application/json" } },
        );
      }
      throw new Error(`unexpected ${url}`);
    });
    vi.stubGlobal("fetch", fetchMock);

    const queryClient = createQueryClient();
    const rootRoute = createRootRoute({ component: () => <Outlet /> });
    const registerRoute = createRoute({
      getParentRoute: () => rootRoute,
      path: "/register",
      component: RegisterPage,
    });
    const homeRoute = createRoute({
      getParentRoute: () => rootRoute,
      path: "/",
      component: () => <div>工作台</div>,
    });
    const router = createRouter({
      routeTree: rootRoute.addChildren([registerRoute, homeRoute]),
      history: createMemoryHistory({ initialEntries: ["/register"] }),
      context: { queryClient },
    });

    render(
      <QueryClientProvider client={queryClient}>
        <RouterProvider router={router} />
      </QueryClientProvider>,
    );

    await userEvent.type(await screen.findByLabelText("邮箱"), "teacher@example.com");
    await userEvent.type(screen.getByLabelText("密码"), "a-long-enough-password");
    await userEvent.type(screen.getByLabelText("邀请码"), "INVITE-123");
    await userEvent.click(screen.getByRole("button", { name: "创建账号" }));

    expect(await screen.findByText("工作台")).toBeInTheDocument();
  });
});
