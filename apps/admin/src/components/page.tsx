import type { ReactNode } from "react";

/** 管理端页面级布局；与 C 端共享 UI 基元分开。 */
export function Page({ title, children }: { title: string; children: ReactNode }) {
  return (
    <main className="page">
      <h1 className="page-title">{title}</h1>
      {children}
    </main>
  );
}
