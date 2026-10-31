// @vitest-environment jsdom
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import "@testing-library/jest-dom/vitest";
import { describe, expect, it, vi } from "vitest";

import { Button } from "./button";

describe("Button", () => {
  it("渲染默认变体并响应点击", async () => {
    const onClick = vi.fn();
    render(<Button onClick={onClick}>保存</Button>);

    const button = screen.getByRole("button", { name: "保存" });
    expect(button).toHaveClass("bg-primary");
    await userEvent.click(button);
    expect(onClick).toHaveBeenCalledTimes(1);
  });
});
