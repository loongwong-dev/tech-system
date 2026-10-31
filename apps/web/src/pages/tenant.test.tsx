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

import { SelectTenantPage } from "./SelectTenantPage";
import { createQueryClient } from "../runtime/queryClient";

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});

describe("SelectTenantPage", () => {
  it("切换到目标租户后重新读取 /me 并进入工作台", async () => {
    vi.spyOn(window, "scrollTo").mockImplementation(() => undefined);
    const fetchMock = vi.fn(async (url: string) => {
      if (url === "/auth/tenants") {
        return new Response(
          JSON.stringify({ items: [{ id: "t2", name: "第二个租户", role: "owner" }] }),
          { status: 200, headers: { "content-type": "application/json" } },
        );
      }
      if (url === "/auth/tenant") {
        return new Response(null, { status: 204 });
      }
      if (url === "/api/v1/me") {
        return new Response(
          JSON.stringify({
            userId: "u1",
            email: "teacher@example.com",
            tenantId: "t2",
            tenantName: "第二个租户",
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
    const selectRoute = createRoute({
      getParentRoute: () => rootRoute,
      path: "/select-tenant",
      component: SelectTenantPage,
    });
    const homeRoute = createRoute({
      getParentRoute: () => rootRoute,
      path: "/",
      component: () => <div>工作台</div>,
    });
    const router = createRouter({
      routeTree: rootRoute.addChildren([selectRoute, homeRoute]),
      history: createMemoryHistory({ initialEntries: ["/select-tenant"] }),
      context: { queryClient },
    });

    render(
      <QueryClientProvider client={queryClient}>
        <RouterProvider router={router} />
      </QueryClientProvider>,
    );

    await userEvent.click(await screen.findByRole("button", { name: /第二个租户/ }));
    expect(await screen.findByText("工作台")).toBeInTheDocument();
    expect(fetchMock).toHaveBeenCalledWith(
      "/auth/tenant",
      expect.objectContaining({ credentials: "include", method: "POST" }),
    );
  });
});
