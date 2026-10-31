import { Suspense, type ReactNode } from "react";
import { Outlet, createRootRouteWithContext, createRoute, createRouter, useRouterState } from "@tanstack/react-router";

import { AppShell } from "./components/app-shell";
import { TodayPage } from "./pages/TodayPage";
import { RouteError, RouteLoading } from "./runtime/boundaries";
import { queryClient } from "./runtime/queryClient";
import { requireSession, requireUser, type RouterContext } from "./runtime/session";
import { LessonPage, LoginPage, RegisterPage, ReviewPage, SelectTenantPage, SettingsPage, StudentsPage } from "./routes/lazy";

const standalonePaths = new Set(["/login", "/register", "/select-tenant"]);

function RootLayout() {
  const pathname = useRouterState({ select: (state) => state.location.pathname });
  if (standalonePaths.has(pathname)) {
    return <Outlet />;
  }
  return (
    <AppShell>
      <Outlet />
    </AppShell>
  );
}

type AuthSearch = { redirect?: string };

function authSearch(search: Record<string, unknown>): AuthSearch {
  return {
    redirect: typeof search.redirect === "string" ? search.redirect : undefined,
  };
}

function LazyPage({ children }: { children: ReactNode }) {
  return <Suspense fallback={<p className="p-6 text-sm text-muted-foreground">加载中…</p>}>{children}</Suspense>;
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

const loginRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/login",
  validateSearch: authSearch,
  component: () => (
    <LazyPage>
      <LoginPage />
    </LazyPage>
  ),
});

const registerRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/register",
  validateSearch: authSearch,
  component: () => (
    <LazyPage>
      <RegisterPage />
    </LazyPage>
  ),
});

const selectTenantRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/select-tenant",
  validateSearch: authSearch,
  beforeLoad: requireUser,
  component: () => (
    <LazyPage>
      <SelectTenantPage />
    </LazyPage>
  ),
});

const routeTree = rootRoute.addChildren([
  indexRoute,
  lessonRoute,
  reviewRoute,
  studentsRoute,
  settingsRoute,
  loginRoute,
  registerRoute,
  selectTenantRoute,
]);

export const router = createRouter({ routeTree, context: { queryClient } });

declare module "@tanstack/react-router" {
  interface Register {
    router: typeof router;
  }
}
