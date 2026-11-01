import { queryOptions } from "@tanstack/react-query";
import type { components } from "@tech-system/api-types";

import { apiFetch } from "../../runtime/api";
import { queryKeys } from "../../runtime/queryKeys";

export type Student = components["schemas"]["StudentResponse"];
export type StudentList = components["schemas"]["StudentListResponse"];
export type Class = components["schemas"]["ClassResponse"];
export type ClassList = components["schemas"]["ClassListResponse"];

export type StudentInput = components["schemas"]["CreateStudentRequest"];
export type StudentUpdateInput = components["schemas"]["UpdateStudentRequest"];
export type ClassInput = components["schemas"]["CreateClassRequest"];
export type ClassUpdateInput = components["schemas"]["UpdateClassRequest"];

export const studentsQuery = queryOptions({
  queryKey: queryKeys.students,
  queryFn: () => apiFetch<StudentList>("/api/v1/students"),
  retry: false,
  staleTime: 15_000,
});

export const classesQuery = queryOptions({
  queryKey: queryKeys.classes,
  queryFn: () => apiFetch<ClassList>("/api/v1/classes"),
  retry: false,
  staleTime: 15_000,
});

/** 创建学员。 */
export function createStudent(input: StudentInput): Promise<Student> {
  return apiFetch<Student>("/api/v1/students", {
    method: "POST",
    body: JSON.stringify(input),
  });
}

/** 更新学员。 */
export function updateStudent(id: string, input: StudentUpdateInput): Promise<Student> {
  return apiFetch<Student>(`/api/v1/students/${id}`, {
    method: "PUT",
    body: JSON.stringify(input),
  });
}

/** 归档学员。 */
export function archiveStudent(id: string): Promise<void> {
  return apiFetch<void>(`/api/v1/students/${id}/archive`, { method: "POST" });
}

/** 姓名确认后删除学员；级联删除由服务端完成。 */
export function deleteStudent(id: string, name: string): Promise<void> {
  return apiFetch<void>(`/api/v1/students/${id}`, {
    method: "DELETE",
    body: JSON.stringify({ name }),
  });
}

/** 创建班级。 */
export function createClass(input: ClassInput): Promise<Class> {
  return apiFetch<Class>("/api/v1/classes", {
    method: "POST",
    body: JSON.stringify(input),
  });
}

/** 更新班级。 */
export function updateClass(id: string, input: ClassUpdateInput): Promise<Class> {
  return apiFetch<Class>(`/api/v1/classes/${id}`, {
    method: "PUT",
    body: JSON.stringify(input),
  });
}

/** 归档班级；归档后不再出现在列表中。 */
export function archiveClass(id: string): Promise<void> {
  return apiFetch<void>(`/api/v1/classes/${id}/archive`, { method: "POST" });
}

/** 删除没有课次的班级；有课次时服务端返回 409。 */
export function deleteClass(id: string): Promise<void> {
  return apiFetch<void>(`/api/v1/classes/${id}`, { method: "DELETE" });
}
