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
  archiveStudent,
  createStudent,
  deleteStudent,
  studentsQuery,
  updateStudent,
  type Student,
  type StudentInput,
} from "./api";

const schema = z.object({
  name: z.string().trim().min(1, "姓名必填").max(50, "姓名不能超过 50 个字符"),
  grade: z.string().trim().max(50, "年级不能超过 50 个字符"),
  subject: z.string().trim().max(50, "科目不能超过 50 个字符"),
});

type StudentValues = z.infer<typeof schema>;

function toInput(values: StudentValues): StudentInput {
  return {
    name: values.name,
    grade: values.grade || null,
    subject: values.subject || null,
  };
}

/** 学员列表与创建、编辑、归档、姓名确认删除。 */
export function StudentSection() {
  const queryClient = useQueryClient();
  const students = useQuery(studentsQuery);
  const [editing, setEditing] = useState<Student | null>(null);
  const [formOpen, setFormOpen] = useState(false);
  const [deleteTarget, setDeleteTarget] = useState<Student | null>(null);
  const [deleteName, setDeleteName] = useState("");
  const [formError, setFormError] = useState<string | null>(null);
  const [deleteError, setDeleteError] = useState<string | null>(null);

  const form = useForm<StudentValues>({
    resolver: zodResolver(schema),
    defaultValues: { name: "", grade: "", subject: "" },
  });

  const save = useMutation({
    mutationFn: (values: StudentValues) =>
      editing
        ? updateStudent(editing.id, toInput(values))
        : createStudent(toInput(values)),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: studentsQuery.queryKey });
      setFormOpen(false);
      setFormError(null);
    },
    onError: (error) => setFormError(reportApiError(error)),
  });

  const archive = useMutation({
    mutationFn: (student: Student) => archiveStudent(student.id),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: studentsQuery.queryKey }),
    onError: (error) => setFormError(reportApiError(error)),
  });

  const remove = useMutation({
    mutationFn: () => {
      if (!deleteTarget) {
        throw new Error("未选择学员");
      }
      return deleteStudent(deleteTarget.id, deleteName);
    },
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: studentsQuery.queryKey });
      setDeleteTarget(null);
      setDeleteName("");
      setDeleteError(null);
    },
    onError: (error) => setDeleteError(reportApiError(error)),
  });

  function openCreate() {
    setEditing(null);
    setFormError(null);
    form.reset({ name: "", grade: "", subject: "" });
    setFormOpen(true);
  }

  function openEdit(student: Student) {
    setEditing(student);
    setFormError(null);
    form.reset({
      name: student.name,
      grade: student.grade ?? "",
      subject: student.subject ?? "",
    });
    setFormOpen(true);
  }

  function openDelete(student: Student) {
    setDeleteTarget(student);
    setDeleteName("");
    setDeleteError(null);
  }

  const errors = form.formState.errors;

  return (
    <section className="flex min-w-0 flex-col gap-4 rounded-xl border border-border bg-card p-4 text-card-foreground">
      <div className="flex items-center justify-between gap-3">
        <h2 className="text-lg font-semibold">学员</h2>
        <Button type="button" size="sm" className="min-h-11" onClick={openCreate}>
          添加学员
        </Button>
      </div>

      {students.isLoading ? <p className="text-sm text-muted-foreground">加载中…</p> : null}
      {students.error ? <p className="text-sm text-destructive" role="alert">{students.error.message}</p> : null}
      {students.data?.items.length === 0 ? (
        <p className="text-sm text-muted-foreground">还没有学员。</p>
      ) : null}

      <ul className="flex flex-col gap-2">
        {students.data?.items.map((student) => (
          <li key={student.id} className="flex flex-col gap-3 rounded-lg border border-border p-3 sm:flex-row sm:items-center sm:justify-between">
            <div className="min-w-0">
              <p className="truncate font-medium">{student.name}</p>
              <p className="truncate text-sm text-muted-foreground">
                {[student.grade, student.subject].filter(Boolean).join(" · ") || "未填写年级/科目"}
              </p>
            </div>
            <div className="flex flex-wrap gap-1">
              <Button type="button" variant="ghost" size="sm" className="min-h-11" onClick={() => openEdit(student)}>
                编辑
              </Button>
              <Button
                type="button"
                variant="ghost"
                size="sm"
                className="min-h-11"
                disabled={archive.isPending}
                onClick={() => archive.mutate(student)}
              >
                归档
              </Button>
              <Button
                type="button"
                variant="ghost"
                size="sm"
                className="min-h-11 text-destructive"
                onClick={() => openDelete(student)}
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
            <DialogTitle>{editing ? "编辑学员" : "添加学员"}</DialogTitle>
            <DialogDescription>只保存姓名、年级与科目，不保存家长联系方式。</DialogDescription>
          </DialogHeader>
          <form className="flex flex-col gap-4" onSubmit={form.handleSubmit((values) => save.mutate(values))}>
            <FieldGroup>
              <Field data-invalid={Boolean(errors.name)}>
                <FieldLabel htmlFor="student-name">姓名</FieldLabel>
                <Input id="student-name" className="h-11" aria-invalid={Boolean(errors.name)} {...form.register("name")} />
                {errors.name?.message ? <FieldError>{errors.name.message}</FieldError> : null}
              </Field>
              <Field data-invalid={Boolean(errors.grade)}>
                <FieldLabel htmlFor="student-grade">年级</FieldLabel>
                <Input id="student-grade" className="h-11" aria-invalid={Boolean(errors.grade)} {...form.register("grade")} />
                {errors.grade?.message ? <FieldError>{errors.grade.message}</FieldError> : null}
              </Field>
              <Field data-invalid={Boolean(errors.subject)}>
                <FieldLabel htmlFor="student-subject">科目</FieldLabel>
                <Input id="student-subject" className="h-11" aria-invalid={Boolean(errors.subject)} {...form.register("subject")} />
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
            <DialogTitle>删除学员</DialogTitle>
            <DialogDescription>
              将级联删除该学员的反馈与分享物。请输入姓名「{deleteTarget?.name}」确认。
            </DialogDescription>
          </DialogHeader>
          <Field data-invalid={Boolean(deleteError)}>
            <FieldLabel htmlFor="delete-student-name">学员姓名</FieldLabel>
            <Input
              id="delete-student-name"
              className="h-11"
              value={deleteName}
              aria-invalid={Boolean(deleteError)}
              onChange={(event) => setDeleteName(event.target.value)}
            />
            {deleteError ? <FieldError>{deleteError}</FieldError> : null}
          </Field>
          <DialogFooter>
            <Button type="button" variant="outline" className="min-h-11" onClick={() => setDeleteTarget(null)}>
              取消
            </Button>
            <Button
              type="button"
              variant="destructive"
              className="min-h-11"
              disabled={remove.isPending || deleteName !== deleteTarget?.name}
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
