import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import { Button } from "@tech-system/ui/components/button";

import { Page } from "../components/page";
import { ApiClientError } from "../runtime/api";
import { authErrorMessage, logout } from "../runtime/auth";
import { meQuery } from "../runtime/session";

export function SettingsPage() {
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const me = useQuery(meQuery);
  const mutation = useMutation({
    mutationFn: async () => {
      try {
        await logout();
      } catch (error) {
        // 会话已经过期时仍视为登出成功，清掉本地状态。
        if (!(error instanceof ApiClientError && error.status === 401)) {
          throw error;
        }
      }
      queryClient.clear();
      await navigate({ to: "/login", replace: true });
    },
  });

  return (
    <Page title="账号与配额">
      {me.isLoading ? <p className="text-sm text-muted-foreground">加载中…</p> : null}
      {me.error ? <p className="text-sm text-destructive" role="alert">{authErrorMessage(me.error)}</p> : null}
      {me.data ? (
        <section className="flex flex-col gap-3 rounded-lg border border-border p-4 text-sm">
          <p><span className="text-muted-foreground">邮箱：</span>{me.data.email}</p>
          <p><span className="text-muted-foreground">租户：</span>{me.data.tenantName}</p>
          <p><span className="text-muted-foreground">角色：</span>{me.data.role}</p>
          <p><span className="text-muted-foreground">本月配额：</span>{me.data.quota.remaining} / {me.data.quota.limit}</p>
        </section>
      ) : null}
      {mutation.error ? <p className="text-sm text-destructive" role="alert">{authErrorMessage(mutation.error)}</p> : null}
      <Button type="button" variant="outline" disabled={mutation.isPending} onClick={() => mutation.mutate()}>
        {mutation.isPending ? "退出中…" : "退出登录"}
      </Button>
    </Page>
  );
}
