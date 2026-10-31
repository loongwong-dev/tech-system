import { ApiClientError } from "./api";

/** 单份生成完成事件；与后端 done JSON 对齐。 */
export type GenerationDone = {
  feedbackId: string;
  text: string;
  generatedBy: "ai" | "template";
  promptVersion: string;
  reused: boolean;
};

/** SSE 生成事件。 */
export type GenerationEvent =
  | { type: "delta"; text: string }
  | { type: "done"; data: GenerationDone };

const DEFAULT_TIMEOUT_MS = 20_000;

/**
 * 读取单份生成 SSE。
 *
 * 按 `delta` 与 `done` 事件解析；超时或外部中断时抛出错误，调用方保留已收到的内容。
 */
export async function* streamGeneration(
  path: string,
  options: { signal?: AbortSignal; timeoutMs?: number } = {},
): AsyncGenerator<GenerationEvent> {
  const controller = new AbortController();
  const timeoutMs = options.timeoutMs ?? DEFAULT_TIMEOUT_MS;
  let timedOut = false;

  const timeout = setTimeout(() => {
    timedOut = true;
    controller.abort();
  }, timeoutMs);
  let reader: ReadableStreamDefaultReader<Uint8Array> | undefined;
  const abortFromCaller = () => controller.abort(options.signal?.reason);
  options.signal?.addEventListener("abort", abortFromCaller, { once: true });
  if (options.signal?.aborted) {
    controller.abort(options.signal.reason);
  }

  try {
    const response = await fetch(path, {
      method: "POST",
      headers: { accept: "text/event-stream" },
      credentials: "include",
      signal: controller.signal,
    });
    if (!response.ok) {
      throw await ApiClientError.fromResponse(response);
    }
    if (!response.body) {
      throw new ApiClientError(0, "NETWORK_EMPTY_STREAM", "生成流为空");
    }

    reader = response.body.getReader();
    const decoder = new TextDecoder();
    let buffer = "";

    for (;;) {
      const { done, value } = await reader.read();
      if (done) {
        break;
      }
      buffer += decoder.decode(value, { stream: true }).replaceAll("\r\n", "\n");
      let boundary = buffer.indexOf("\n\n");
      while (boundary >= 0) {
        const block = buffer.slice(0, boundary);
        buffer = buffer.slice(boundary + 2);
        const event = parseEvent(block);
        if (event) {
          yield event;
          if (event.type === "done") {
            return;
          }
        }
        boundary = buffer.indexOf("\n\n");
      }
    }
  } catch (error) {
    if (timedOut) {
      throw new ApiClientError(0, "NETWORK_TIMEOUT", "生成超时");
    }
    throw error;
  } finally {
    clearTimeout(timeout);
    options.signal?.removeEventListener("abort", abortFromCaller);
    // consumer 提前 break 或收到 done 后，关闭底层响应体，避免连接悬挂。
    if (reader) {
      await reader.cancel().catch(() => undefined);
    }
  }
}

function parseEvent(block: string): GenerationEvent | undefined {
  let eventName = "message";
  const data: string[] = [];

  for (const line of block.split("\n")) {
    if (line.startsWith(":")) {
      continue;
    }
    if (line.startsWith("event:")) {
      eventName = line.slice("event:".length).trim();
    } else if (line.startsWith("data:")) {
      data.push(line.slice("data:".length).replace(/^ /, ""));
    }
  }

  const payload = data.join("\n");
  if (eventName === "delta") {
    return { type: "delta", text: payload };
  }
  if (eventName === "done") {
    return { type: "done", data: JSON.parse(payload) as GenerationDone };
  }
  return undefined;
}
