import type { ReactNode } from "react";

/** C 端页面级布局；不进入共享 UI 包。 */
export function Page({ title, children }: { title: string; children: ReactNode }) {
  return (
    <main className="grid gap-4">
      <h1 className="m-0 text-2xl font-semibold leading-tight">{title}</h1>
      {children}
    </main>
  );
}
