import { afterEach, describe, expect, it, vi } from "vitest";

import { streamGeneration } from "./sse";

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("streamGeneration", () => {
  it("按 delta 与 done 解析，不丢 done.text", async () => {
    const body = [
      "event: delta",
      "data: 课堂",
      "",
      "event: delta",
      "data: 表现",
      "",
      "event: done",
      'data: {"feedbackId":"f1","text":"课堂表现","generatedBy":"template","promptVersion":"v1","reused":false}',
      "",
      "",
    ].join("\n");
    const stream = new ReadableStream({
      start(controller) {
        controller.enqueue(new TextEncoder().encode(body));
        controller.close();
      },
    });
    vi.stubGlobal(
      "fetch",
      vi.fn(async () => new Response(stream, { status: 200 })),
    );

    const events = [];
    for await (const event of streamGeneration("/api/v1/feedbacks/f1/generations")) {
      events.push(event);
    }

    expect(events).toEqual([
      { type: "delta", text: "课堂" },
      { type: "delta", text: "表现" },
      {
        type: "done",
        data: {
          feedbackId: "f1",
          text: "课堂表现",
          generatedBy: "template",
          promptVersion: "v1",
          reused: false,
        },
      },
    ]);
  });

  it("超时返回 NETWORK_TIMEOUT", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn(
        (_path: string, init: RequestInit) =>
          new Promise((_resolve, reject) => {
            init.signal?.addEventListener("abort", () => {
              reject(new DOMException("aborted", "AbortError"));
            });
          }),
      ),
    );

    await expect(async () => {
      for await (const _event of streamGeneration("/generations", { timeoutMs: 10 })) {
        // 不应收到事件。
      }
    }).rejects.toMatchObject({ code: "NETWORK_TIMEOUT", status: 0 });
  });
});
