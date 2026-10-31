import { Suspense, type ReactNode } from "react";
import { Outlet, createRootRouteWithContext, createRoute, createRouter } from "@tanstack/react-router";

import { TodayPage } from "./pages/TodayPage";
import { RouteError, RouteLoading } from "./runtime/boundaries";
import { queryClient } from "./runtime/queryClient";
import { requireSession, type RouterContext } from "./runtime/session";
import { LessonPage, ReviewPage, SettingsPage, StudentsPage } from "./routes/lazy";

function RootLayout() {
  return (
    <div className="app-shell">
      <Outlet />
    </div>
  );
}

function LazyPage({ children }: { children: ReactNode }) {
  return <Suspense fallback={<p className="page-loading">加载中…</p>}>{children}</Suspense>;
}

const rootRoute = createRootRouteWithContext<RouterContext>()({
  component: RootLayout,
  errorComponent: RouteError,
  pendingComponent: RouteLoading,
});

const indexRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/",
  component: TodayPage,
  beforeLoad: requireSession,
});

const lessonRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/lessons/$id",
  component: () => (
    <LazyPage>
      <LessonPage />
    </LazyPage>
  ),
  beforeLoad: requireSession,
});

const reviewRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/lessons/$id/review",
  component: () => (
    <LazyPage>
      <ReviewPage />
    </LazyPage>
  ),
  beforeLoad: requireSession,
});

const studentsRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/students",
  component: () => (
    <LazyPage>
      <StudentsPage />
    </LazyPage>
  ),
  beforeLoad: requireSession,
});

const settingsRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/settings",
  component: () => (
    <LazyPage>
      <SettingsPage />
    </LazyPage>
  ),
  beforeLoad: requireSession,
});

const routeTree = rootRoute.addChildren([
  indexRoute,
  lessonRoute,
  reviewRoute,
  studentsRoute,
  settingsRoute,
]);

export const router = createRouter({ routeTree, context: { queryClient } });

declare module "@tanstack/react-router" {
  interface Register {
    router: typeof router;
  }
}
