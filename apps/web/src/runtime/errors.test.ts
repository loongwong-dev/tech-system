import { beforeEach, describe, expect, it, vi } from "vitest";

import { toast } from "@tech-system/ui/components/sonner";

import { ApiClientError } from "./api";
import { reportApiError } from "./errors";

vi.mock("@tech-system/ui/components/sonner", () => ({
  toast: { error: vi.fn() },
}));

function problem(status: number, detail: string) {
  return {
    type: "https://example.com/errors/test",
    title: "请求失败",
    status,
    detail,
    instance: null,
    code: "TEST",
    errors: [],
  };
}

beforeEach(() => {
  vi.clearAllMocks();
});

describe("reportApiError", () => {
  it.each([409, 429])("%i 使用 Toast 展示 detail", (status) => {
    const detail = `服务端错误 ${status}`;
    const error = new ApiClientError(status, "TEST", detail, [], problem(status, detail));

    expect(reportApiError(error)).toBeNull();
    expect(toast.error).toHaveBeenCalledWith(detail);
  });

  it("普通错误返回消息，不弹 Toast", () => {
    const error = new ApiClientError(500, "TEST", "服务器错误", [], problem(500, "服务器错误"));

    expect(reportApiError(error)).toBe("服务器错误");
    expect(toast.error).not.toHaveBeenCalled();
  });
});
