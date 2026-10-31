import { zodResolver } from "@hookform/resolvers/zod";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { Link, useNavigate, useSearch } from "@tanstack/react-router";
import { Button } from "@tech-system/ui/components/button";
import { Field, FieldError, FieldGroup, FieldLabel } from "@tech-system/ui/components/field";
import { Input } from "@tech-system/ui/components/input";
import { useForm } from "react-hook-form";
import { z } from "zod";

import { AuthLayout } from "../components/auth-layout";
import { authErrorMessage, login, safeRedirect } from "../runtime/auth";
import { meQuery } from "../runtime/session";

const schema = z.object({
  email: z.string().trim().email("请输入有效邮箱"),
  password: z.string().min(1, "请输入密码"),
});

type LoginValues = z.infer<typeof schema>;

export function LoginPage() {
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const { redirect } = useSearch({ from: "/login" });
  const form = useForm<LoginValues>({
    resolver: zodResolver(schema),
    defaultValues: { email: "", password: "" },
  });
  const mutation = useMutation({
    mutationFn: async ({ email, password }: LoginValues) => {
      const result = await login(email, password);
      // 清除上一个会话的用户/租户缓存，避免共享浏览器中串用旧数据。
      queryClient.clear();
      if (!result.tenantId) {
        await navigate({ to: "/select-tenant", search: { redirect }, replace: true });
        return;
      }
      await queryClient.fetchQuery(meQuery);
      await navigate({ href: safeRedirect(redirect), replace: true });
    },
  });

  const errors = form.formState.errors;

  return (
    <AuthLayout
      title="登录教师账号"
      description="登录后继续录入和生成课后反馈。"
      footer={
        <>
          还没有账号？ <Link to="/register" className="font-medium text-foreground underline-offset-4 hover:underline">使用邀请码注册</Link>
        </>
      }
    >
      <form
        className="flex flex-col gap-4"
        onSubmit={form.handleSubmit((values) => mutation.mutate(values))}
      >
        <FieldGroup>
          <Field data-invalid={Boolean(errors.email)}>
            <FieldLabel htmlFor="email">邮箱</FieldLabel>
            <Input id="email" type="email" autoComplete="email" aria-invalid={Boolean(errors.email)} {...form.register("email")} />
            {errors.email?.message ? <FieldError>{errors.email.message}</FieldError> : null}
          </Field>
          <Field data-invalid={Boolean(errors.password)}>
            <FieldLabel htmlFor="password">密码</FieldLabel>
            <Input id="password" type="password" autoComplete="current-password" aria-invalid={Boolean(errors.password)} {...form.register("password")} />
            {errors.password?.message ? <FieldError>{errors.password.message}</FieldError> : null}
          </Field>
        </FieldGroup>
        {mutation.error ? <p className="text-sm text-destructive" role="alert">{authErrorMessage(mutation.error)}</p> : null}
        <Button type="submit" disabled={mutation.isPending}>
          {mutation.isPending ? "登录中…" : "登录"}
        </Button>
      </form>
    </AuthLayout>
  );
}
