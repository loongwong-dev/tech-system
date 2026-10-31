import { existsSync, readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { gzipSync } from "node:zlib";

export const DEFAULT_BUDGET_BYTES = 150 * 1024;
export const DEFAULT_MANIFEST = "apps/web/dist/.vite/manifest.json";

/**
 * 计算首屏 JS 的 gzip 体积。
 *
 * 只沿 manifest 的静态 `imports` 递归，不包含 `dynamicImports`，
 * 因此懒加载路由不会计入首屏预算。
 */
export function firstScreenGzipBytes(manifestPath) {
  const manifest = JSON.parse(readFileSync(manifestPath, "utf8"));
  const outDir = resolve(dirname(manifestPath), "..");
  const entry = Object.entries(manifest).find(([, value]) => value.isEntry);
  if (!entry) {
    throw new Error(`manifest 中没有 entry: ${manifestPath}`);
  }

  const seen = new Set();
  let total = 0;
  const visit = (key) => {
    if (seen.has(key)) {
      return;
    }
    seen.add(key);
    const item = manifest[key];
    if (!item) {
      return;
    }
    total += gzipSync(readFileSync(join(outDir, item.file))).byteLength;
    for (const imported of item.imports ?? []) {
      visit(imported);
    }
  };

  visit(entry[0]);
  return total;
}

export function main() {
  const [manifestArg, budgetArg, labelArg] = process.argv.slice(2);
  const manifestPath = manifestArg ?? process.env.SIZE_MANIFEST ?? DEFAULT_MANIFEST;
  const budget = Number(budgetArg ?? process.env.SIZE_BUDGET_BYTES ?? DEFAULT_BUDGET_BYTES);
  const label = labelArg ?? process.env.SIZE_LABEL ?? "web";

  if (!Number.isFinite(budget) || budget < 0) {
    console.error("预算必须是非负数字");
    process.exitCode = 1;
    return;
  }

  if (!existsSync(manifestPath)) {
    console.error(`未找到 ${manifestPath}，请先运行 bun run build`);
    process.exitCode = 1;
    return;
  }

  const bytes = firstScreenGzipBytes(manifestPath);
  console.log(`${label} 首屏 JS：${bytes} bytes gzip（预算 ${budget} bytes）`);
  if (bytes > budget) {
    console.error(`超出预算 ${bytes - budget} bytes`);
    process.exitCode = 1;
  }
}

const isMain = import.meta.main || process.argv[1] === fileURLToPath(import.meta.url);
if (isMain) {
  main();
}
