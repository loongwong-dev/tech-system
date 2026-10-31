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

import { LoginPage } from "./LoginPage";
import { createQueryClient } from "../runtime/queryClient";

function renderLogin(fetchMock: ReturnType<typeof vi.fn>, initialPath = "/login") {
  vi.spyOn(window, "scrollTo").mockImplementation(() => undefined);
  vi.stubGlobal("fetch", fetchMock);
  const queryClient = createQueryClient();
  const rootRoute = createRootRoute({ component: () => <Outlet /> });
  const loginRoute = createRoute({
    getParentRoute: () => rootRoute,
    path: "/login",
    component: LoginPage,
  });
  const homeRoute = createRoute({
    getParentRoute: () => rootRoute,
    path: "/",
    component: () => <div>工作台</div>,
  });
  const studentsRoute = createRoute({
    getParentRoute: () => rootRoute,
    path: "/students",
    component: () => <div>学员页</div>,
  });
  const router = createRouter({
    routeTree: rootRoute.addChildren([loginRoute, homeRoute, studentsRoute]),
    history: createMemoryHistory({ initialEntries: [initialPath] }),
    context: { queryClient },
  });

  render(
    <QueryClientProvider client={queryClient}>
      <RouterProvider router={router} />
    </QueryClientProvider>,
  );
}

function jsonResponse(body: unknown, status = 200) {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "content-type": "application/json" },
  });
}

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});

describe("LoginPage", () => {
  it("登录成功后刷新会话并进入工作台", async () => {
    const fetchMock = vi.fn(async (url: string) => {
      if (url === "/auth/login") {
        return jsonResponse({ userId: "u1", tenantId: "t1" });
      }
      if (url === "/api/v1/me") {
        return jsonResponse({
          userId: "u1",
          email: "teacher@example.com",
          tenantId: "t1",
          tenantName: "租户",
          role: "owner",
          quota: { limit: 15, used: 0, remaining: 15, resetAt: "2026-11-01T00:00:00Z" },
        });
      }
      throw new Error(`unexpected ${url}`);
    });
    renderLogin(fetchMock);

    await userEvent.type(await screen.findByLabelText("邮箱"), "teacher@example.com");
    await userEvent.type(screen.getByLabelText("密码"), "a-long-enough-password");
    await userEvent.click(screen.getByRole("button", { name: "登录" }));

    expect(await screen.findByText("工作台")).toBeInTheDocument();
    expect(fetchMock).toHaveBeenCalledWith(
      "/auth/login",
      expect.objectContaining({ credentials: "include", method: "POST" }),
    );
  });

  it("登录失败显示错误码对应文案", async () => {
    const fetchMock = vi.fn(async () =>
      new Response(
        JSON.stringify({ code: "AUTH_INVALID_CREDENTIALS", detail: "invalid" }),
        { status: 401, headers: { "content-type": "application/problem+json" } },
      ),
    );
    renderLogin(fetchMock);

    await userEvent.type(await screen.findByLabelText("邮箱"), "teacher@example.com");
    await userEvent.type(screen.getByLabelText("密码"), "wrong-password");
    await userEvent.click(screen.getByRole("button", { name: "登录" }));

    expect(await screen.findByRole("alert")).toHaveTextContent("邮箱或密码错误");
  });
});

describe("Login redirect", () => {
  it("登录后回到安全的本站 redirect", async () => {
    const fetchMock = vi.fn(async (url: string) => {
      if (url === "/auth/login") {
        return jsonResponse({ userId: "u1", tenantId: "t1" });
      }
      if (url === "/api/v1/me") {
        return jsonResponse({
          userId: "u1",
          email: "teacher@example.com",
          tenantId: "t1",
          tenantName: "租户",
          role: "owner",
          quota: { limit: 15, used: 0, remaining: 15, resetAt: "2026-11-01T00:00:00Z" },
        });
      }
      throw new Error(`unexpected ${url}`);
    });
    renderLogin(fetchMock, "/login?redirect=%2Fstudents");

    await userEvent.type(await screen.findByLabelText("邮箱"), "teacher@example.com");
    await userEvent.type(screen.getByLabelText("密码"), "a-long-enough-password");
    await userEvent.click(screen.getByRole("button", { name: "登录" }));

    expect(await screen.findByText("学员页")).toBeInTheDocument();
  });
});
