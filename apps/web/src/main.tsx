import { StrictMode, Suspense, lazy } from "react";
import { QueryClientProvider } from "@tanstack/react-query";
import { RouterProvider } from "@tanstack/react-router";
import { createRoot } from "react-dom/client";

import "./styles/global.css";
import { setUnauthorizedHandler } from "./runtime/api";
import { queryClient } from "./runtime/queryClient";
import { router } from "./router";

const Devtools = import.meta.env.DEV
  ? lazy(() =>
      import("@tanstack/react-query-devtools").then((module) => ({
        default: module.ReactQueryDevtools,
      })),
    )
  : null;

setUnauthorizedHandler(() => {
  queryClient.clear();
  void router.navigate({ to: "/login", replace: true });
});

const root = document.getElementById("root");
if (!root) {
  throw new Error("缺少 #root 挂载点");
}

createRoot(root).render(
  <StrictMode>
    <QueryClientProvider client={queryClient}>
      <RouterProvider router={router} />
      {Devtools ? (
        <Suspense fallback={null}>
          <Devtools initialIsOpen={false} />
        </Suspense>
      ) : null}
    </QueryClientProvider>
  </StrictMode>,
);
