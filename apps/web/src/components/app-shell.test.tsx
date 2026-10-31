// @vitest-environment jsdom
import { render, screen } from "@testing-library/react";
import { RouterProvider, createMemoryHistory, createRootRoute, createRoute, createRouter } from "@tanstack/react-router";
import "@testing-library/jest-dom/vitest";
import { describe, expect, it, vi } from "vitest";

import { AppShell } from "./app-shell";

function buildRouter() {
  const rootRoute = createRootRoute({
    component: () => (
      <AppShell>
        <div>页面内容</div>
      </AppShell>
    ),
  });
  const indexRoute = createRoute({
    getParentRoute: () => rootRoute,
    path: "/",
    component: () => <div>今日内容</div>,
  });
  const studentsRoute = createRoute({
    getParentRoute: () => rootRoute,
    path: "/students",
    component: () => <div>学员内容</div>,
  });
  const settingsRoute = createRoute({
    getParentRoute: () => rootRoute,
    path: "/settings",
    component: () => <div>设置内容</div>,
  });
  return createRouter({
    routeTree: rootRoute.addChildren([indexRoute, studentsRoute, settingsRoute]),
    history: createMemoryHistory({ initialEntries: ["/"] }),
  });
}

describe("AppShell", () => {
  it("渲染桌面与移动端导航", async () => {
    vi.spyOn(window, "scrollTo").mockImplementation(() => undefined);
    render(<RouterProvider router={buildRouter()} />);

    expect(await screen.findByText("页面内容")).toBeInTheDocument();
    expect(screen.getAllByRole("navigation")).toHaveLength(2);
    expect(screen.getAllByRole("link", { name: "今日" })).toHaveLength(2);
    expect(screen.getAllByRole("link", { name: "学员" })).toHaveLength(2);
    expect(screen.getAllByRole("link", { name: "设置" })).toHaveLength(2);
  });
});
