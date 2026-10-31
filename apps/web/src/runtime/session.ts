import { queryOptions, type QueryClient } from "@tanstack/react-query";
import { redirect } from "@tanstack/react-router";
import type { components } from "@tech-system/api-types";

import { apiFetch, ApiClientError } from "./api";
import { queryKeys } from "./queryKeys";

/** 当前会话与配额。 */
export type Me = components["schemas"]["MeResponse"];

/** 路由 context 中的运行时依赖。 */
export type RouterContext = {
  queryClient: QueryClient;
};

/** 当前会话查询；由路由守卫与页面共享同一缓存。 */
export const meQuery = queryOptions({
  queryKey: queryKeys.me,
  queryFn: () => apiFetch<Me>("/api/v1/me"),
  retry: false,
  staleTime: 60_000,
});

/**
 * 受保护路由的 beforeLoad：确保会话与租户上下文存在。
 *
 * `/me` 只通过 Query 缓存请求；未登录与未选租户分别跳转登录页和租户选择页。
 */
export async function requireSession({
  context,
  location,
}: {
  context: RouterContext;
  location: { href: string };
}): Promise<Me> {
  try {
    const me = await context.queryClient.ensureQueryData(meQuery);
    if (!me.tenantId) {
      // TODO(#90): 路由注册后移除 never 断言；当前只建立守卫，页面由 #90 提供。
      throw redirect({
        to: "/select-tenant",
        search: { redirect: location.href },
      } as never);
    }
    return me;
  } catch (error) {
    if (error instanceof ApiClientError && error.status === 401) {
      // 清除 401 错误缓存，避免登录成功后仍被旧结果重定向到登录页。
      context.queryClient.removeQueries({ queryKey: queryKeys.me });
      // TODO(#90): 路由注册后移除 never 断言；当前只建立守卫，页面由 #90 提供。
      throw redirect({ to: "/login", search: { redirect: location.href } } as never);
    }
    throw error;
  }
}
