// 校验任意 JSON 文档是否符合 schemas/*.schema.json（SX-009 起：CLI 输出的独立复核入口）。
// 用法（在 apps/desktop 目录）：
//   node scripts/contract/validate-document.mjs --schema batch --file <路径>
// 退出码：0 通过；1 未通过；2 用法错误。
import { readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import Ajv2020 from "ajv/dist/2020.js";
import addFormats from "ajv-formats";
import $RefParser from "@apidevtools/json-schema-ref-parser";

const here = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(here, "../../../..");
const schemasDir = path.join(repoRoot, "schemas");

const args = process.argv.slice(2);
const readArg = (name) => {
  const index = args.indexOf(name);
  return index >= 0 ? args[index + 1] : undefined;
};
const schema = readArg("--schema");
const file = readArg("--file");
if (!schema || !file) {
  console.error("用法：node scripts/contract/validate-document.mjs --schema <名称> --file <路径>");
  process.exit(2);
}

const ajv = new Ajv2020({ strict: true, allErrors: true });
addFormats(ajv);
const bundled = await $RefParser.bundle(path.join(schemasDir, `${schema}.schema.json`));
const validate = ajv.compile(bundled);
const document = JSON.parse(readFileSync(file, "utf8"));
if (validate(document)) {
  console.log(`ok  ${file} 通过 ${schema} schema`);
  process.exit(0);
}
console.error(`FAIL  ${file} 未通过 ${schema} schema`);
console.error(JSON.stringify(validate.errors?.slice(0, 5), null, 2));
process.exit(1);
