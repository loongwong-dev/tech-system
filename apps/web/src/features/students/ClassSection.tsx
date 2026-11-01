import { zodResolver } from "@hookform/resolvers/zod";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { useForm } from "react-hook-form";
import { z } from "zod";

import { Button } from "@tech-system/ui/components/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@tech-system/ui/components/dialog";
import { Field, FieldError, FieldGroup, FieldLabel } from "@tech-system/ui/components/field";
import { Input } from "@tech-system/ui/components/input";

import { reportApiError } from "../../runtime/errors";
import {
  classesQuery,
  createClass,
  deleteClass,
  updateClass,
  type Class,
  type ClassInput,
} from "./api";

const schema = z.object({
  name: z.string().trim().min(1, "名称必填").max(50, "名称不能超过 50 个字符"),
  subject: z.string().trim().max(50, "科目不能超过 50 个字符"),
});

type ClassValues = z.infer<typeof schema>;

function toInput(values: ClassValues): ClassInput {
  return { name: values.name, subject: values.subject || null };
}

/** 班级列表与创建、编辑、删除。 */
export function ClassSection() {
  const queryClient = useQueryClient();
  const classes = useQuery(classesQuery);
  const [editing, setEditing] = useState<Class | null>(null);
  const [formOpen, setFormOpen] = useState(false);
  const [deleteTarget, setDeleteTarget] = useState<Class | null>(null);
  const [formError, setFormError] = useState<string | null>(null);
  const [deleteError, setDeleteError] = useState<string | null>(null);

  const form = useForm<ClassValues>({
    resolver: zodResolver(schema),
    defaultValues: { name: "", subject: "" },
  });

  const save = useMutation({
    mutationFn: (values: ClassValues) =>
      editing ? updateClass(editing.id, toInput(values)) : createClass(toInput(values)),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: classesQuery.queryKey });
      setFormOpen(false);
      setFormError(null);
    },
    onError: (error) => setFormError(reportApiError(error)),
  });

  const remove = useMutation({
    mutationFn: () => {
      if (!deleteTarget) {
        throw new Error("未选择班级");
      }
      return deleteClass(deleteTarget.id);
    },
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: classesQuery.queryKey });
      setDeleteTarget(null);
      setDeleteError(null);
    },
    onError: (error) => setDeleteError(reportApiError(error)),
  });

  function openCreate() {
    setEditing(null);
    setFormError(null);
    form.reset({ name: "", subject: "" });
    setFormOpen(true);
  }

  function openEdit(item: Class) {
    setEditing(item);
    setFormError(null);
    form.reset({ name: item.name, subject: item.subject ?? "" });
    setFormOpen(true);
  }

  const errors = form.formState.errors;

  return (
    <section className="flex min-w-0 flex-col gap-4 rounded-xl border border-border bg-card p-4 text-card-foreground">
      <div className="flex items-center justify-between gap-3">
        <h2 className="text-lg font-semibold">班级</h2>
        <Button type="button" size="sm" className="min-h-11" onClick={openCreate}>
          添加班级
        </Button>
      </div>

      {classes.isLoading ? <p className="text-sm text-muted-foreground">加载中…</p> : null}
      {classes.error ? <p className="text-sm text-destructive" role="alert">{classes.error.message}</p> : null}
      {classes.data?.items.length === 0 ? (
        <p className="text-sm text-muted-foreground">还没有班级。</p>
      ) : null}

      <ul className="flex flex-col gap-2">
        {classes.data?.items.map((item) => (
          <li key={item.id} className="flex flex-col gap-3 rounded-lg border border-border p-3 sm:flex-row sm:items-center sm:justify-between">
            <div className="min-w-0">
              <p className="truncate font-medium">{item.name}</p>
              <p className="truncate text-sm text-muted-foreground">{item.subject || "未填写科目"}</p>
            </div>
            <div className="flex flex-wrap gap-1">
              <Button type="button" variant="ghost" size="sm" className="min-h-11" onClick={() => openEdit(item)}>
                编辑
              </Button>
              <Button
                type="button"
                variant="ghost"
                size="sm"
                className="min-h-11 text-destructive"
                onClick={() => {
                  setDeleteTarget(item);
                  setDeleteError(null);
                }}
              >
                删除
              </Button>
            </div>
          </li>
        ))}
      </ul>

      <Dialog open={formOpen} onOpenChange={setFormOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{editing ? "编辑班级" : "添加班级"}</DialogTitle>
            <DialogDescription>删除有课次的班级会被拒绝。</DialogDescription>
          </DialogHeader>
          <form className="flex flex-col gap-4" onSubmit={form.handleSubmit((values) => save.mutate(values))}>
            <FieldGroup>
              <Field data-invalid={Boolean(errors.name)}>
                <FieldLabel htmlFor="class-name">名称</FieldLabel>
                <Input id="class-name" className="h-11" aria-invalid={Boolean(errors.name)} {...form.register("name")} />
                {errors.name?.message ? <FieldError>{errors.name.message}</FieldError> : null}
              </Field>
              <Field data-invalid={Boolean(errors.subject)}>
                <FieldLabel htmlFor="class-subject">科目</FieldLabel>
                <Input id="class-subject" className="h-11" aria-invalid={Boolean(errors.subject)} {...form.register("subject")} />
                {errors.subject?.message ? <FieldError>{errors.subject.message}</FieldError> : null}
              </Field>
            </FieldGroup>
            {formError ? <p className="text-sm text-destructive" role="alert">{formError}</p> : null}
            <DialogFooter>
              <Button type="button" variant="outline" className="min-h-11" onClick={() => setFormOpen(false)}>
                取消
              </Button>
              <Button type="submit" className="min-h-11" disabled={save.isPending}>
                {save.isPending ? "保存中…" : "保存"}
              </Button>
            </DialogFooter>
          </form>
        </DialogContent>
      </Dialog>

      <Dialog open={Boolean(deleteTarget)} onOpenChange={(open) => !open && setDeleteTarget(null)}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>删除班级</DialogTitle>
            <DialogDescription>
              确认删除「{deleteTarget?.name}」？如果班级仍有课次，服务端会拒绝并提示原因。
            </DialogDescription>
          </DialogHeader>
          {deleteError ? <p className="text-sm text-destructive" role="alert">{deleteError}</p> : null}
          <DialogFooter>
            <Button type="button" variant="outline" className="min-h-11" onClick={() => setDeleteTarget(null)}>
              取消
            </Button>
            <Button
              type="button"
              variant="destructive"
              className="min-h-11"
              disabled={remove.isPending}
              onClick={() => remove.mutate()}
            >
              {remove.isPending ? "删除中…" : "确认删除"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </section>
  );
}
