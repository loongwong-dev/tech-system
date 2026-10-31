import type { ReactNode } from "react";

/** 认证页面独立布局；不渲染 AppShell 导航。 */
export function AuthLayout({
  title,
  description,
  children,
  footer,
}: {
  title: string;
  description: string;
  children: ReactNode;
  footer?: ReactNode;
}) {
  return (
    <main className="flex min-h-dvh items-center justify-center bg-muted/40 px-4 py-10">
      <section className="w-full max-w-sm rounded-xl border border-border bg-card p-6 text-card-foreground shadow-sm">
        <div className="mb-6 flex flex-col gap-1">
          <h1 className="text-xl font-semibold tracking-tight">{title}</h1>
          <p className="text-sm text-muted-foreground">{description}</p>
        </div>
        {children}
        {footer ? <div className="mt-6 border-t border-border pt-4 text-sm text-muted-foreground">{footer}</div> : null}
      </section>
    </main>
  );
}
