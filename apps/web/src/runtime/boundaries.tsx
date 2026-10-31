import { useRouter, type ErrorComponentProps } from "@tanstack/react-router";

/** 路由错误边界；保留错误信息与重试入口。 */
export function RouteError({ error, reset }: ErrorComponentProps) {
  const router = useRouter();
  return (
    <section className="page" role="alert">
      <h1 className="page-title">页面加载失败</h1>
      <p>{error instanceof Error ? error.message : "未知错误"}</p>
      <button
        type="button"
        onClick={() => {
          reset();
          void router.invalidate();
        }}
      >
        重试
      </button>
    </section>
  );
}

/** 路由 pending 组件；避免切换时闪空页面。 */
export function RouteLoading() {
  return <p className="page-loading">加载中…</p>;
}
