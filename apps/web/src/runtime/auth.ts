import { queryOptions } from "@tanstack/react-query";
import type { components } from "@tech-system/api-types";

import { apiFetch, ApiClientError } from "./api";
import { queryKeys } from "./queryKeys";

/** 登录响应。 */
export type LoginResponse = components["schemas"]["LoginResponse"];
/** 注册响应。 */
export type RegisterResponse = components["schemas"]["RegisterResponse"];
/** 可切换租户。 */
export type TenantSummary = components["schemas"]["TenantSummaryResponse"];
/** 租户列表响应。 */
export type TenantListResponse = components["schemas"]["TenantListResponse"];

/** 当前用户可切换的租户列表；会话 cookie 即可访问。 */
export const tenantsQuery = queryOptions({
  queryKey: queryKeys.tenants,
  queryFn: () => apiFetch<TenantListResponse>("/auth/tenants"),
  retry: false,
  staleTime: 30_000,
});

/** 邮箱密码登录；成功后 Cookie 由服务端轮换。 */
export function login(email: string, password: string): Promise<LoginResponse> {
  return apiFetch<LoginResponse>("/auth/login", {
    method: "POST",
    body: JSON.stringify({ email, password }),
  });
}

/** 邀请码注册并建立会话。 */
export function register(input: {
  email: string;
  password: string;
  inviteCode: string;
}): Promise<RegisterResponse> {
  return apiFetch<RegisterResponse>("/auth/register", {
    method: "POST",
    body: JSON.stringify(input),
  });
}

/** 切换租户；成功后服务端轮换会话。 */
export function switchTenant(tenantId: string): Promise<void> {
  return apiFetch<void>("/auth/tenant", {
    method: "POST",
    body: JSON.stringify({ tenantId }),
  });
}

/** 吊销当前会话。 */
export function logout(): Promise<void> {
  return apiFetch<void>("/auth/logout", { method: "POST" });
}

/** 将认证错误码转换为用户可读文案。 */
export function authErrorMessage(error: unknown): string {
  if (error instanceof ApiClientError) {
    switch (error.code) {
      case "AUTH_INVALID_CREDENTIALS":
        return "邮箱或密码错误";
      case "AUTH_SESSION_EXPIRED":
        return "会话已失效，请重新登录";
      case "AUTH_TENANT_REQUIRED":
        return "请选择租户";
      case "CONFLICT_EMAIL_EXISTS":
        return "该邮箱已注册";
      case "CONFLICT_INVITE_CODE_INVALID":
        return "邀请码无效或已用完";
      case "FORBIDDEN_TENANT":
        return "无法切换到该租户";
      case "VALIDATION_INVALID":
      case "VALIDATION_NAME_MISMATCH":
        return "请检查填写内容";
      default:
        return error.message;
    }
  }
  return "网络异常，请稍后重试";
}

/** 仅允许回到本站的非认证路径，避免开放重定向。 */
export function safeRedirect(value: string | undefined): string {
  if (!value) {
    return "/";
  }
  try {
    const url = new URL(value, window.location.origin);
    if (url.origin !== window.location.origin) {
      return "/";
    }
    if (["/login", "/register", "/select-tenant"].includes(url.pathname)) {
      return "/";
    }
    return `${url.pathname}${url.search}${url.hash}`;
  } catch {
    return "/";
  }
}
