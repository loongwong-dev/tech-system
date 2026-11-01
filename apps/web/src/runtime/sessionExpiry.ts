import type { QueryClient } from "@tanstack/react-query";

import { setUnauthorizedHandler } from "./api";

type LoginNavigator = {
  navigate: (options: { to: "/login"; replace: true }) => unknown;
};

/** 安装会话失效处理：清空缓存并跳转登录。 */
export function installSessionExpiryHandler(
  queryClient: QueryClient,
  navigator: LoginNavigator,
): void {
  setUnauthorizedHandler(() => {
    queryClient.clear();
    void navigator.navigate({ to: "/login", replace: true });
  });
}
