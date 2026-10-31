import type { components } from "@tech-system/api-types";

/** 后端 Problem Details 的生成类型。 */
export type ProblemDetails = components["schemas"]["ProblemDetails"];
/** 字段级校验错误。 */
export type FieldError = components["schemas"]["FieldError"];

/** 请求失败；保留稳定错误码与字段错误。 */
export class ApiClientError extends Error {
  readonly status: number;
  readonly code: string;
  readonly errors: FieldError[];
  readonly problem?: ProblemDetails;

  constructor(
    status: number,
    code: string,
    detail: string,
    errors: FieldError[] = [],
    problem?: ProblemDetails,
  ) {
    super(detail);
    this.name = "ApiClientError";
    this.status = status;
    this.code = code;
    this.errors = errors;
    this.problem = problem;
  }

  /** 从非 2xx 响应构造错误；解析失败时退化为 HTTP 状态码。 */
  static async fromResponse(response: Response): Promise<ApiClientError> {
    const contentType = response.headers.get("content-type") ?? "";
    let problem: ProblemDetails | undefined;
    if (contentType.includes("application/problem+json") || contentType.includes("application/json")) {
      try {
        problem = (await response.json()) as ProblemDetails;
      } catch {
        problem = undefined;
      }
    }

    return new ApiClientError(
      response.status,
      problem?.code ?? `HTTP_${response.status}`,
      problem?.detail ?? (response.statusText || "请求失败"),
      problem?.errors ?? [],
      problem,
    );
  }
}

/** 发送 JSON API 请求；携带 HttpOnly 会话 Cookie。 */
export async function apiFetch<T>(path: string, init: RequestInit = {}): Promise<T> {
  const headers = new Headers(init.headers);
  headers.set("accept", "application/json");
  if (typeof init.body === "string" && !headers.has("content-type")) {
    headers.set("content-type", "application/json");
  }

  const response = await fetch(path, {
    ...init,
    headers,
    credentials: "include",
  });

  if (!response.ok) {
    throw await ApiClientError.fromResponse(response);
  }
  if (response.status === 204) {
    return undefined as T;
  }
  return (await response.json()) as T;
}
