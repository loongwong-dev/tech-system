import { zodResolver } from "@hookform/resolvers/zod";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { Link, useNavigate, useSearch } from "@tanstack/react-router";
import { Button } from "@tech-system/ui/components/button";
import { Field, FieldDescription, FieldError, FieldGroup, FieldLabel } from "@tech-system/ui/components/field";
import { Input } from "@tech-system/ui/components/input";
import { useForm } from "react-hook-form";
import { z } from "zod";

import { AuthLayout } from "../components/auth-layout";
import { authErrorMessage, register, safeRedirect } from "../runtime/auth";
import { meQuery } from "../runtime/session";

const schema = z.object({
  email: z.string().trim().email("请输入有效邮箱"),
  password: z.string().min(10, "密码至少 10 位").max(128, "密码最多 128 位"),
  inviteCode: z.string().trim().min(1, "请输入邀请码"),
});

type RegisterValues = z.infer<typeof schema>;

export function RegisterPage() {
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const { redirect } = useSearch({ from: "/register" });
  const form = useForm<RegisterValues>({
    resolver: zodResolver(schema),
    defaultValues: { email: "", password: "", inviteCode: "" },
  });
  const mutation = useMutation({
    mutationFn: (values: RegisterValues) => register(values),
    onSuccess: async () => {
      // 清除上一个会话的用户/租户缓存，避免共享浏览器中串用旧数据。
      queryClient.clear();
      await queryClient.fetchQuery(meQuery);
      await navigate({ to: "/", replace: true });
    },
  });

  const errors = form.formState.errors;

  return (
    <AuthLayout
      title="创建教师账号"
      description="使用邀请码创建个人租户。"
      footer={
        <>
          已有账号？ <Link to="/login" className="font-medium text-foreground underline-offset-4 hover:underline">直接登录</Link>
        </>
      }
    >
      <form className="flex flex-col gap-4" onSubmit={form.handleSubmit((values) => mutation.mutate(values))}>
        <FieldGroup>
          <Field data-invalid={Boolean(errors.email)}>
            <FieldLabel htmlFor="email">邮箱</FieldLabel>
            <Input id="email" type="email" autoComplete="email" aria-invalid={Boolean(errors.email)} {...form.register("email")} />
            {errors.email?.message ? <FieldError>{errors.email.message}</FieldError> : null}
          </Field>
          <Field data-invalid={Boolean(errors.password)}>
            <FieldLabel htmlFor="password">密码</FieldLabel>
            <Input id="password" type="password" autoComplete="new-password" aria-invalid={Boolean(errors.password)} {...form.register("password")} />
            <FieldDescription>至少 10 位。</FieldDescription>
            {errors.password?.message ? <FieldError>{errors.password.message}</FieldError> : null}
          </Field>
          <Field data-invalid={Boolean(errors.inviteCode)}>
            <FieldLabel htmlFor="inviteCode">邀请码</FieldLabel>
            <Input id="inviteCode" autoComplete="one-time-code" aria-invalid={Boolean(errors.inviteCode)} {...form.register("inviteCode")} />
            {errors.inviteCode?.message ? <FieldError>{errors.inviteCode.message}</FieldError> : null}
          </Field>
        </FieldGroup>
        {mutation.error ? <p className="text-sm text-destructive" role="alert">{authErrorMessage(mutation.error)}</p> : null}
        <Button type="submit" disabled={mutation.isPending}>
          {mutation.isPending ? "创建中…" : "创建账号"}
        </Button>
      </form>
    </AuthLayout>
  );
}
