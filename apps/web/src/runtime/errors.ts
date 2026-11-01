import { toast } from "@tech-system/ui/components/sonner";

import { ApiClientError } from "./api";

/**
 * 将业务冲突与限流错误以 Toast 展示 detail；返回表单可展示的普通错误。
 */
export function reportApiError(error: unknown): string | null {
  if (error instanceof ApiClientError) {
    if (error.status === 409 || error.status === 429) {
      toast.error(error.problem?.detail ?? error.message);
      return null;
    }
    return error.message;
  }
  return error instanceof Error ? error.message : "操作失败";
}
