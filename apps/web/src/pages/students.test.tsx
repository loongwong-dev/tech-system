// @vitest-environment jsdom
import { QueryClientProvider } from "@tanstack/react-query";
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import "@testing-library/jest-dom/vitest";
import { afterEach, describe, expect, it, vi } from "vitest";

import { StudentsPage } from "./StudentsPage";
import { createQueryClient } from "../runtime/queryClient";

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});

describe("StudentsPage", () => {
  it("删除前要求输入姓名，确认后调用删除接口", async () => {
    vi.spyOn(window, "scrollTo").mockImplementation(() => undefined);
    const fetchMock = vi.fn(async (url: string, init?: RequestInit) => {
      if (url === "/api/v1/students" && (!init?.method || init.method === "GET")) {
        return new Response(
          JSON.stringify({
            items: [{ id: "s1", name: "小明", grade: "初二", subject: "数学" }],
          }),
          { status: 200, headers: { "content-type": "application/json" } },
        );
      }
      if (url === "/api/v1/classes") {
        return new Response(JSON.stringify({ items: [] }), {
          status: 200,
          headers: { "content-type": "application/json" },
        });
      }
      if (url === "/api/v1/students/s1" && init?.method === "DELETE") {
        return new Response(JSON.stringify({ id: "s1", feedbackCount: 0 }), {
          status: 200,
          headers: { "content-type": "application/json" },
        });
      }
      throw new Error(`unexpected ${url}`);
    });
    vi.stubGlobal("fetch", fetchMock);

    render(
      <QueryClientProvider client={createQueryClient()}>
        <StudentsPage />
      </QueryClientProvider>,
    );

    await userEvent.click(await screen.findByRole("button", { name: "删除" }));
    const confirmButton = screen.getByRole("button", { name: "确认删除" });
    expect(confirmButton).toBeDisabled();

    await userEvent.type(screen.getByLabelText("学员姓名"), "小明");
    expect(confirmButton).toBeEnabled();
    await userEvent.click(confirmButton);

    await waitFor(() => {
      expect(fetchMock).toHaveBeenCalledWith(
        "/api/v1/students/s1",
        expect.objectContaining({ method: "DELETE" }),
      );
    });
  });
});
