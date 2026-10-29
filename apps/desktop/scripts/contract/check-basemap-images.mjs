// P2-07 图片清单校验：按 data/map/basemaps.json 核对图片存在性、字节数、SHA-256
// 以及 WebP 实际像素尺寸是否与 image_width / image_height 一致。
//
// 用法（在 apps/desktop 目录）：
//   pnpm contract:images [--root <图片根目录>]
// 根目录解析顺序：--root → 环境变量 BASEMAP_IMAGE_ROOT → image_root 约定位置。
// image_root.kind = bundled 时默认 data/map/<path>，external 时默认仓库外的
// ../../resource/map-processed（构建机约定，其它环境请显式传 --root）。
import { createHash } from "node:crypto";
import { existsSync, readFileSync, statSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const here = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(here, "../../../..");
const datasetPath = path.join(repoRoot, "data/map/basemaps.json");

function argValue(flag) {
  const index = process.argv.indexOf(flag);
  return index >= 0 ? process.argv[index + 1] : undefined;
}

/** 从 WebP 容器头读出画布尺寸；无法识别时返回 null。 */
function webpSize(buffer) {
  if (buffer.length < 30) return null;
  if (buffer.toString("ascii", 0, 4) !== "RIFF") return null;
  if (buffer.toString("ascii", 8, 12) !== "WEBP") return null;
  let offset = 12;
  while (offset + 8 <= buffer.length) {
    const fourcc = buffer.toString("ascii", offset, offset + 4);
    const size = buffer.readUInt32LE(offset + 4);
    const data = offset + 8;
    if (data + size > buffer.length) return null;
    if (fourcc === "VP8X") {
      const width = 1 + (buffer[data + 4] | (buffer[data + 5] << 8) | (buffer[data + 6] << 16));
      const height = 1 + (buffer[data + 7] | (buffer[data + 8] << 8) | (buffer[data + 9] << 16));
      return { width, height };
    }
    if (fourcc === "VP8L") {
      const bits = buffer.readUInt32LE(data + 1);
      return { width: 1 + (bits & 0x3fff), height: 1 + ((bits >> 14) & 0x3fff) };
    }
    if (fourcc === "VP8 ") {
      const start = data + 3;
      if (buffer[start] === 0x9d && buffer[start + 1] === 0x01 && buffer[start + 2] === 0x2a) {
        return {
          width: buffer.readUInt16LE(start + 3) & 0x3fff,
          height: buffer.readUInt16LE(start + 5) & 0x3fff,
        };
      }
    }
    offset = data + size + (size & 1);
  }
  return null;
}

const dataset = JSON.parse(readFileSync(datasetPath, "utf8"));
const { image_root: imageRoot, basemaps } = dataset;

const root =
  argValue("--root") ??
  process.env.BASEMAP_IMAGE_ROOT ??
  (imageRoot.kind === "bundled"
    ? path.join(repoRoot, "data/map", imageRoot.path)
    : path.resolve(repoRoot, "../../resource/map-processed"));

if (!existsSync(root)) {
  console.error(
    `图片根目录不存在：${root}\n` +
      `image_root.kind=${imageRoot.kind}；请用 --root 或 BASEMAP_IMAGE_ROOT 指定。`,
  );
  process.exit(2);
}

let failures = 0;
let checked = 0;
let totalBytes = 0;
const problems = [];

for (const [key, entry] of Object.entries(basemaps)) {
  const file = path.join(root, entry.image);
  checked += 1;
  if (!existsSync(file)) {
    problems.push(`${key}: 缺图 ${entry.image}`);
    failures += 1;
    continue;
  }
  const bytes = statSync(file).size;
  totalBytes += bytes;
  if (entry.image_bytes != null && bytes !== entry.image_bytes) {
    problems.push(`${key}: 字节数 ${bytes} ≠ ${entry.image_bytes}`);
    failures += 1;
  }
  const buffer = readFileSync(file);
  const digest = createHash("sha256").update(buffer).digest("hex");
  if (entry.image_sha256 != null && digest !== entry.image_sha256) {
    problems.push(`${key}: sha256 ${digest.slice(0, 12)}… ≠ ${entry.image_sha256.slice(0, 12)}…`);
    failures += 1;
  }
  if (file.endsWith(".webp")) {
    const size = webpSize(buffer);
    if (size == null) {
      problems.push(`${key}: 无法读取 WebP 尺寸`);
      failures += 1;
    } else if (size.width !== entry.image_width || size.height !== entry.image_height) {
      problems.push(
        `${key}: 实际 ${size.width}×${size.height} ≠ 声明 ${entry.image_width}×${entry.image_height}`,
      );
      failures += 1;
    }
  }
}

console.log(`图片根目录：${root}（${imageRoot.kind}）`);
console.log(`已核对 ${checked} 张，合计 ${(totalBytes / 1e6).toFixed(1)} MB`);

if (failures > 0) {
  console.error(`${failures} 项不一致：`);
  for (const problem of problems.slice(0, 20)) console.error(`  ${problem}`);
  process.exit(1);
}
console.log("图片清单校验通过");
