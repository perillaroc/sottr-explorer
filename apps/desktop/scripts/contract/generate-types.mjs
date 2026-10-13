// SX-003 前端类型生成：由 schemas/*.schema.json 生成 TypeScript 类型（唯一契约来源）。
// 用法（在 apps/desktop 目录）：pnpm contract:types
import { mkdirSync, writeFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import $RefParser from "@apidevtools/json-schema-ref-parser";
import { compile } from "json-schema-to-typescript";

const here = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(here, "../../../..");
const schemasDir = path.join(repoRoot, "schemas");
const outDir = path.resolve(here, "../../src/contract");

const targets = [
  { schema: "report", name: "Report" },
  { schema: "metadata", name: "MetadataDataset" },
  { schema: "batch", name: "BatchReport" },
  { schema: "desktop-result", name: "DesktopResult" },
  { schema: "export", name: "ExportDocument" },
];

mkdirSync(outDir, { recursive: true });

for (const { schema, name } of targets) {
  const bundled = await $RefParser.bundle(path.join(schemasDir, `${schema}.schema.json`));
  const types = await compile(bundled, name, {
    bannerComment: `/* 由 apps/desktop/scripts/contract/generate-types.mjs 从 schemas/${schema}.schema.json 生成，请勿手改。契约说明见 docs/contracts.md。 */`,
  });
  writeFileSync(path.join(outDir, `${schema}.d.ts`), types);
  console.log(`generated src/contract/${schema}.d.ts`);
}
