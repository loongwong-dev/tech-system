// @vitest-environment jsdom
import { render, screen } from "@testing-library/react";
import "@testing-library/jest-dom/vitest";
import { describe, expect, it } from "vitest";

import { Field, FieldError, FieldLabel } from "./field";
import { Input } from "./input";

describe("Field", () => {
  it("渲染标签并关联错误状态", () => {
    render(
      <Field data-invalid>
        <FieldLabel htmlFor="name">姓名</FieldLabel>
        <Input id="name" aria-invalid />
        <FieldError>姓名必填</FieldError>
      </Field>,
    );

    expect(screen.getByLabelText("姓名")).toHaveAttribute("aria-invalid", "true");
    expect(screen.getByRole("alert")).toHaveTextContent("姓名必填");
    expect(screen.getByText("姓名必填")).toBeInTheDocument();
  });
});
