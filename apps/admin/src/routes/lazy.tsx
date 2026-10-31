import { lazy } from "react";

export const TenantsPage = lazy(() =>
  import("../pages/TenantsPage").then((module) => ({ default: module.TenantsPage })),
);
export const TenantDetailPage = lazy(() =>
  import("../pages/TenantDetailPage").then((module) => ({ default: module.TenantDetailPage })),
);
export const InviteCodesPage = lazy(() =>
  import("../pages/InviteCodesPage").then((module) => ({ default: module.InviteCodesPage })),
);
export const AuditPage = lazy(() =>
  import("../pages/AuditPage").then((module) => ({ default: module.AuditPage })),
);
