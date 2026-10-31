import type { ReactNode } from "react";

/** 页面基础壳：统一标题、内容间距与可读宽度。 */
export function Page({ title, children }: { title: string; children: ReactNode }) {
  return (
    <main className="page">
      <h1 className="page-title">{title}</h1>
      {children}
    </main>
  );
}
