import { QueryClient } from "@tanstack/react-query";

import { ApiClientError } from "./api";

/** 创建应用 QueryClient；401/4xx 不重试，5xx 最多重试一次。 */
export function createQueryClient(): QueryClient {
  return new QueryClient({
    defaultOptions: {
      queries: {
        staleTime: 30_000,
        refetchOnWindowFocus: false,
        retry: (failureCount, error) => {
          if (error instanceof ApiClientError && error.status < 500) {
            return false;
          }
          return failureCount < 1;
        },
      },
    },
  });
}

/** C 端单例；测试使用 createQueryClient 避免共享缓存。 */
export const queryClient = createQueryClient();
