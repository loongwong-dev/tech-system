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

    await userEvent.click(await screen.findByRole("button", { name: "删除学员 小明" }));
    const confirmButton = screen.getByRole("button", { name: "确认删除" });
    expect(confirmButton).toBeDisabled();

    const nameInput = screen.getByLabelText("学员姓名");
    await userEvent.type(nameInput, "小");
    expect(confirmButton).toBeDisabled();
    await userEvent.type(nameInput, "明");
    expect(confirmButton).toBeEnabled();
    await userEvent.click(confirmButton);

    await waitFor(() => {
      expect(fetchMock).toHaveBeenCalledWith(
        "/api/v1/students/s1",
        expect.objectContaining({
          method: "DELETE",
          body: JSON.stringify({ name: "小明" }),
        }),
      );
    });
  });

  it("归档班级调用归档接口", async () => {
    const fetchMock = vi.fn(async (url: string, init?: RequestInit) => {
      if (url === "/api/v1/students" && (!init?.method || init.method === "GET")) {
        return new Response(JSON.stringify({ items: [] }), {
          status: 200,
          headers: { "content-type": "application/json" },
        });
      }
      if (url === "/api/v1/classes" && (!init?.method || init.method === "GET")) {
        return new Response(JSON.stringify({ items: [{ id: "c1", name: "初二数学班", subject: "数学" }] }), {
          status: 200,
          headers: { "content-type": "application/json" },
        });
      }
      if (url === "/api/v1/classes/c1/archive" && init?.method === "POST") {
        return new Response(null, { status: 204 });
      }
      throw new Error(`unexpected ${url}`);
    });
    vi.stubGlobal("fetch", fetchMock);

    render(
      <QueryClientProvider client={createQueryClient()}>
        <StudentsPage />
      </QueryClientProvider>,
    );

    await userEvent.click(await screen.findByRole("button", { name: "归档班级 初二数学班" }));
    await waitFor(() => {
      expect(fetchMock).toHaveBeenCalledWith(
        "/api/v1/classes/c1/archive",
        expect.objectContaining({ method: "POST" }),
      );
    });
  });

  it("归档失败时在列表上显示错误", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn(async (url: string, init?: RequestInit) => {
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
        if (url === "/api/v1/students/s1/archive" && init?.method === "POST") {
          return new Response(JSON.stringify({ detail: "归档失败" }), {
            status: 500,
            headers: { "content-type": "application/problem+json" },
          });
        }
        throw new Error(`unexpected ${url}`);
      }),
    );

    render(
      <QueryClientProvider client={createQueryClient()}>
        <StudentsPage />
      </QueryClientProvider>,
    );

    await userEvent.click(await screen.findByRole("button", { name: "归档学员 小明" }));
    expect(await screen.findByText("归档失败")).toBeInTheDocument();
  });
});
