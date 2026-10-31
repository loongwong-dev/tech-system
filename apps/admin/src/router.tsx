import { Suspense, type ReactNode } from "react";
import { Outlet, createRootRoute, createRoute, createRouter } from "@tanstack/react-router";

import { HealthPage } from "./pages/HealthPage";
import { AuditPage, InviteCodesPage, TenantDetailPage, TenantsPage } from "./routes/lazy";

function RootLayout() {
  return (
    <div className="admin-shell">
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
  component: HealthPage,
});

const tenantsRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/tenants",
  component: () => (
    <LazyPage>
      <TenantsPage />
    </LazyPage>
  ),
});

const tenantDetailRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/tenants/$id",
  component: () => (
    <LazyPage>
      <TenantDetailPage />
    </LazyPage>
  ),
});

const inviteCodesRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/invite-codes",
  component: () => (
    <LazyPage>
      <InviteCodesPage />
    </LazyPage>
  ),
});

const auditRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/audit",
  component: () => (
    <LazyPage>
      <AuditPage />
    </LazyPage>
  ),
});

const routeTree = rootRoute.addChildren([
  indexRoute,
  tenantsRoute,
  tenantDetailRoute,
  inviteCodesRoute,
  auditRoute,
]);

export const router = createRouter({ routeTree });

declare module "@tanstack/react-router" {
  interface Register {
    router: typeof router;
  }
}
