import type { ReactNode } from "react";
import { Link } from "@tanstack/react-router";
import { Toaster } from "@tech-system/ui/components/sonner";

const navItems = [
  { to: "/", label: "今日" },
  { to: "/students", label: "学员" },
  { to: "/settings", label: "设置" },
] as const;

/** C 端应用外壳：移动端单列底部导航，桌面端侧栏 + 内容双列。 */
export function AppShell({ children }: { children: ReactNode }) {
  return (
    <div className="min-h-dvh bg-background text-foreground">
      <div className="mx-auto flex min-h-dvh w-full max-w-6xl">
        <aside className="hidden w-56 shrink-0 flex-col gap-8 border-r border-border px-4 py-6 lg:flex">
          <Link to="/" className="px-2 text-base font-semibold tracking-tight">
            教师反馈
          </Link>
          <nav aria-label="主导航" className="flex flex-col gap-1">
            {navItems.map((item) => (
              <Link
                key={item.to}
                to={item.to}
                activeOptions={{ exact: item.to === "/" }}
                className="rounded-md px-3 py-2 text-sm text-muted-foreground hover:bg-accent hover:text-accent-foreground"
                activeProps={{ className: "bg-accent text-accent-foreground" }}
              >
                {item.label}
              </Link>
            ))}
          </nav>
        </aside>

        <div className="min-w-0 flex-1">
          <header className="sticky top-0 z-40 border-b border-border bg-background/95 backdrop-blur lg:hidden">
            <div className="flex h-14 w-full items-center justify-between px-4 md:px-6">
              <Link to="/" className="text-base font-semibold tracking-tight">
                教师反馈
              </Link>
              <nav aria-label="主导航" className="hidden items-center gap-1 md:flex">
                {navItems.map((item) => (
                  <Link
                    key={item.to}
                    to={item.to}
                    activeOptions={{ exact: item.to === "/" }}
                    className="rounded-md px-3 py-2 text-sm text-muted-foreground hover:bg-accent hover:text-accent-foreground"
                    activeProps={{ className: "bg-accent text-accent-foreground" }}
                  >
                    {item.label}
                  </Link>
                ))}
              </nav>
            </div>
          </header>

          <main className="mx-auto w-full max-w-3xl px-4 pb-24 pt-6 md:px-6 md:pb-10 lg:px-8 lg:pt-10">
            {children}
          </main>
        </div>
      </div>

      <nav
        aria-label="移动端主导航"
        className="fixed inset-x-0 bottom-0 z-40 border-t border-border bg-background/95 pb-[env(safe-area-inset-bottom)] backdrop-blur md:hidden"
      >
        <div className="mx-auto flex max-w-3xl">
          {navItems.map((item) => (
            <Link
              key={item.to}
              to={item.to}
              activeOptions={{ exact: item.to === "/" }}
              className="flex flex-1 flex-col items-center gap-1 px-2 py-3 text-xs text-muted-foreground"
              activeProps={{ className: "text-foreground" }}
            >
              {item.label}
            </Link>
          ))}
        </div>
      </nav>
      <Toaster position="top-center" />
    </div>
  );
}
