import { expectTypeOf, test } from "vitest";

import type { components } from "./index";

test("OpenAPI 生成的学员类型可被前端消费", () => {
  expectTypeOf<components["schemas"]["StudentResponse"]>().toMatchTypeOf<{
    id: string;
    name: string;
  }>();
});
