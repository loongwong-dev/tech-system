import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useNavigate, useSearch } from "@tanstack/react-router";
import { Button } from "@tech-system/ui/components/button";

import { AuthLayout } from "../components/auth-layout";
import { authErrorMessage, safeRedirect, switchTenant, tenantsQuery } from "../runtime/auth";
import { meQuery } from "../runtime/session";

export function SelectTenantPage() {
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const { redirect } = useSearch({ from: "/select-tenant" });
  const tenants = useQuery(tenantsQuery);
  const mutation = useMutation({
    mutationFn: switchTenant,
    onSuccess: async () => {
      queryClient.clear();
      await queryClient.fetchQuery(meQuery);
      await navigate({ href: safeRedirect(redirect), replace: true });
    },
  });

  return (
    <AuthLayout title="选择租户" description="你属于多个租户，请选择本次工作的租户。">
      {tenants.isLoading ? <p className="text-sm text-muted-foreground">加载中…</p> : null}
      {tenants.error ? <p className="text-sm text-destructive" role="alert">{authErrorMessage(tenants.error)}</p> : null}
      <div className="flex flex-col gap-2">
        {tenants.data?.items.length === 0 ? (
          <p className="text-sm text-muted-foreground">没有可切换的租户。</p>
        ) : null}
        {tenants.data?.items.map((tenant) => (
          <Button
            key={tenant.id}
            type="button"
            variant="outline"
            className="justify-between"
            disabled={mutation.isPending}
            onClick={() => mutation.mutate(tenant.id)}
          >
            <span>{tenant.name}</span>
            <span className="text-xs text-muted-foreground">{tenant.role}</span>
          </Button>
        ))}
      </div>
      {mutation.error ? <p className="text-sm text-destructive" role="alert">{authErrorMessage(mutation.error)}</p> : null}
    </AuthLayout>
  );
}
