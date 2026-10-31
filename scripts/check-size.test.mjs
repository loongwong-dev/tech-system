import { spawnSync } from "node:child_process";
import { randomBytes } from "node:crypto";
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { afterEach, describe, expect, it } from "vitest";

import { DEFAULT_BUDGET_BYTES, firstScreenGzipBytes } from "./check-size.mjs";

const tempDirs = [];

afterEach(() => {
  for (const dir of tempDirs) {
    rmSync(dir, { recursive: true, force: true });
  }
  tempDirs.length = 0;
});

function writeOversizeFixture() {
  const dir = mkdtempSync(join(tmpdir(), "tech-system-size-"));
  tempDirs.push(dir);
  const viteDir = join(dir, ".vite");
  mkdirSync(viteDir);
  writeFileSync(join(dir, "main.js"), randomBytes(200 * 1024));
  const manifestPath = join(viteDir, "manifest.json");
  writeFileSync(
    manifestPath,
    JSON.stringify({
      "src/main.tsx": { file: "main.js", isEntry: true, imports: [] },
    }),
  );
  return manifestPath;
}

describe("first-screen size budget", () => {
  it("returns bytes above the default budget for an oversize fixture", () => {
    const manifestPath = writeOversizeFixture();
    expect(firstScreenGzipBytes(manifestPath)).toBeGreaterThan(DEFAULT_BUDGET_BYTES);
  });

  it("exits non-zero when the fixture exceeds the configured budget", () => {
    const manifestPath = writeOversizeFixture();
    const result = spawnSync(process.execPath, ["scripts/check-size.mjs"], {
      env: {
        ...process.env,
        SIZE_MANIFEST: manifestPath,
        SIZE_BUDGET_BYTES: "1",
      },
      encoding: "utf8",
    });

    expect(result.status).toBe(1);
  });
});
