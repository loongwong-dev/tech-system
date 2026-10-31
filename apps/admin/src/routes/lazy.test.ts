import { describe, expect, it } from "vitest";

import { AuditPage, InviteCodesPage, TenantDetailPage, TenantsPage } from "./lazy";

const lazySymbol = Symbol.for("react.lazy");

describe("admin lazy routes", () => {
  it("keeps non-home pages as React.lazy components", () => {
    for (const page of [TenantsPage, TenantDetailPage, InviteCodesPage, AuditPage]) {
      expect(page.$$typeof).toBe(lazySymbol);
    }
  });
});
