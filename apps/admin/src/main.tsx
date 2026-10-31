import { StrictMode } from "react";
import { RouterProvider } from "@tanstack/react-router";
import { createRoot } from "react-dom/client";

import "./styles/global.css";
import { router } from "./router";

const root = document.getElementById("root");
if (!root) {
  throw new Error("缺少 #root 挂载点");
}

createRoot(root).render(
  <StrictMode>
    <RouterProvider router={router} />
  </StrictMode>,
);
