import { describe, expect, it } from "vitest";

import { LessonPage, ReviewPage, SettingsPage, StudentsPage } from "./lazy";

const lazySymbol = Symbol.for("react.lazy");

describe("web lazy routes", () => {
  it("keeps non-home pages as React.lazy components", () => {
    for (const page of [LessonPage, ReviewPage, StudentsPage, SettingsPage]) {
      expect(page.$$typeof).toBe(lazySymbol);
    }
  });
});
