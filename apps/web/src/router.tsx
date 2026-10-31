import { Suspense, type ReactNode } from "react";
import { Outlet, createRootRoute, createRoute, createRouter } from "@tanstack/react-router";

import { TodayPage } from "./pages/TodayPage";
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

const rootRoute = createRootRoute({ component: RootLayout });

const indexRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/",
  component: TodayPage,
});

const lessonRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/lessons/$id",
  component: () => (
    <LazyPage>
      <LessonPage />
    </LazyPage>
  ),
});

const reviewRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/lessons/$id/review",
  component: () => (
    <LazyPage>
      <ReviewPage />
    </LazyPage>
  ),
});

const studentsRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/students",
  component: () => (
    <LazyPage>
      <StudentsPage />
    </LazyPage>
  ),
});

const settingsRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/settings",
  component: () => (
    <LazyPage>
      <SettingsPage />
    </LazyPage>
  ),
});

const routeTree = rootRoute.addChildren([
  indexRoute,
  lessonRoute,
  reviewRoute,
  studentsRoute,
  settingsRoute,
]);

export const router = createRouter({ routeTree });

declare module "@tanstack/react-router" {
  interface Register {
    router: typeof router;
  }
}
