# @tech-system/api-types

该包由 `openapi-typescript` 从后端生成的 `openapi.json` 产出，不得手写接口类型。

```bash
bash scripts/generate-openapi.sh
```

生成物：`src/schema.d.ts`。类型从 `@tech-system/api-types` 导入。
