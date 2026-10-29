// SX-003 契约验证：合法夹具必须通过 schema、非法夹具必须被拒绝、u64 表示边界检查。
// 用法（在 apps/desktop 目录）：pnpm contract:validate
import { readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import Ajv2020 from "ajv/dist/2020.js";
import addFormats from "ajv-formats";
import $RefParser from "@apidevtools/json-schema-ref-parser";

const here = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(here, "../../../..");
const schemasDir = path.join(repoRoot, "schemas");
const fixturesDir = path.join(repoRoot, "fixtures", "contract");

const ajv = new Ajv2020({ strict: true, allErrors: true });
addFormats(ajv);

let failures = 0;

function check(ok, label, detail = "") {
  if (ok) {
    console.log(`  ok  ${label}`);
  } else {
    failures += 1;
    console.error(`FAIL  ${label}${detail ? ` — ${detail}` : ""}`);
  }
}

async function validatorFor(schemaName) {
  const bundled = await $RefParser.bundle(path.join(schemasDir, `${schemaName}.schema.json`));
  return ajv.compile(bundled);
}

function load(relative) {
  return JSON.parse(readFileSync(path.join(fixturesDir, relative), "utf8"));
}

const validTargets = [
  ["report", "valid/report.valid.json"],
  ["report", "valid/report.degraded.json"],
  ["report", "valid/report.nonzero-group.json"],
  ["metadata", "valid/metadata.valid.json"],
  ["batch", "valid/batch.mixed.json"],
  ["desktop-result", "valid/desktop-result.current.json"],
  ["desktop-result", "valid/desktop-result.stale.json"],
  ["desktop-result", "valid/desktop-result.cancelled.json"],
  ["desktop-result", "valid/desktop-result.other-slot.json"],
  ["export", "valid/export.stale.json"],
  // SX-008：仓库内真实元数据文件同样由 metadata schema 校验（与 Rust 加载器互补）。
  ["metadata", "../../data/metadata/collectibles.json"],
  ["metadata", "../../data/metadata/region_counters.json"],
  ["metadata", "../../data/metadata/reference_sets.json"],
  // SX-022：全收集图鉴（schema 2.0.0）同样受契约校验（与 Rust catalog 加载器互补）。
  ["catalog", "../../data/catalog/collectibles_catalog.json"],
  // P2-01/P2-05：真实地图基底层同样受契约校验（与 Rust basemap 加载器互补）。
  ["basemap", "../../data/map/basemaps.json"],
];

const invalidTargets = [
  ["report", "invalid/report.bad-observation.json"],
  ["report", "invalid/report.null-value-without-reason.json"],
  ["report", "invalid/report.verified-coordinates-null.json"],
  ["metadata", "invalid/metadata.bad-url.json"],
];

const validators = new Map();

for (const [schemaName, relative] of validTargets) {
  if (!validators.has(schemaName)) {
    validators.set(schemaName, await validatorFor(schemaName));
  }
  const validate = validators.get(schemaName);
  const ok = validate(load(relative));
  check(ok, `${relative} 通过 ${schemaName} schema`, ok ? "" : JSON.stringify(validate.errors?.slice(0, 3)));
}

for (const [schemaName, relative] of invalidTargets) {
  if (!validators.has(schemaName)) {
    validators.set(schemaName, await validatorFor(schemaName));
  }
  const validate = validators.get(schemaName);
  const passed = validate(load(relative));
  check(!passed, `${relative} 被 ${schemaName} schema 拒绝`, passed ? "但通过了校验" : "");
}

// u64 表示规则：只允许字符串形式，十进制与十六进制必须无损一致。
const expected = [18446744073709551615n, 9007199254740993n];
const boundary = load("representation/u64-boundary.json");
boundary.samples.forEach((sample, index) => {
  const ok =
    typeof sample.decimal === "string" &&
    typeof sample.hex === "string" &&
    BigInt(sample.decimal) === expected[index] &&
    BigInt(sample.hex) === expected[index];
  check(ok, `u64 边界字符串无损：${sample.decimal}`);
});

const badNumber = load("representation/u64-as-number.json");
let rejected = false;
try {
  if (typeof badNumber.decimal !== "string") {
    throw new Error("资源键必须是字符串");
  }
} catch {
  rejected = true;
}
check(rejected, "u64 数字形式被拒绝（不会经 JavaScript Number）");

if (failures > 0) {
  console.error(`contract validation failed: ${failures} 项`);
  process.exit(1);
}
console.log(`contract validation passed: ${validTargets.length} 合法 + ${invalidTargets.length} 非法 + 表示边界`);
