/** 全局 Query key 约定；页面不直接拼字符串。 */
export const queryKeys = {
  me: ["me"] as const,
  tenants: ["auth", "tenants"] as const,
  students: ["students"] as const,
  classes: ["classes"] as const,
} as const;
