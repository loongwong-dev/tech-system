import { Button } from "@tech-system/ui/components/button";
import { useRouter, type ErrorComponentProps } from "@tanstack/react-router";

import { ApiClientError } from "./api";

/** 面向用户的路由错误文案，避免暴露内部细节。 */
function readableError(error: unknown): string {
  if (error instanceof ApiClientError) {
    if (error.code === "HTTP_INVALID_JSON") {
      return "后端返回了无法解析的响应。请确认 API 服务已启动，并检查开发代理 VITE_API_TARGET 是否指向正确端口。";
    }
    if (error.code === "NETWORK_TIMEOUT" || error.code === "NETWORK_EMPTY_STREAM") {
      return "连接服务器超时，请稍后重试。";
    }
    return error.message;
  }
  return error instanceof Error ? error.message : "未知错误";
}

/** 路由错误边界；保留错误信息与重试入口。 */
export function RouteError({ error, reset }: ErrorComponentProps) {
  const router = useRouter();
  return (
    <main className="flex min-h-dvh items-center justify-center bg-muted/40 px-4 py-10">
      <section
        className="w-full max-w-md rounded-xl border border-border bg-card p-6 text-card-foreground shadow-sm"
        role="alert"
      >
        <h1 className="text-xl font-semibold tracking-tight">页面加载失败</h1>
        <p className="mt-2 text-sm leading-6 text-muted-foreground">{readableError(error)}</p>
        <Button
          type="button"
          className="mt-5"
          onClick={() => {
            reset();
            void router.invalidate();
          }}
        >
          重试
        </Button>
      </section>
    </main>
  );
}

/** 路由 pending 组件；避免切换时闪空页面。 */
export function RouteLoading() {
  return (
    <main className="flex min-h-dvh items-center justify-center bg-muted/30 px-4">
      <p className="text-sm text-muted-foreground">加载中…</p>
    </main>
  );
}
