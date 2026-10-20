// 展示格式化（SX-015；2026-10-20 增加语言模式）：集中管理数字、时间、大小与 ID 文案。
import { currentLanguage } from "./i18n";

/** 数值/单位文案：中英对照不重复整段数字，按英文或中文其中一种输出。 */
function u(zh: string, en: string): string {
  return currentLanguage() === "en" ? en : zh;
}

/** 百分比：保留两位小数（未知由调用方决定文案，不在这里显示破折号）。 */
export function formatPercent(value: number | null): string {
  if (value === null || !Number.isFinite(value)) {
    return "";
  }
  return `${value.toFixed(2)}%`;
}

/** 字节数：KiB/MiB 便于阅读，同时给出精确字节数供详情使用。 */
export function formatBytes(bytes: number | null): string {
  if (bytes === null) {
    return u("未知", "Unknown");
  }
  if (bytes < 1024) {
    return u(`${bytes} 字节`, `${bytes} B`);
  }
  if (bytes < 1024 * 1024) {
    return u(
      `${(bytes / 1024).toFixed(1)} KiB（${bytes.toLocaleString("zh-CN")} 字节）`,
      `${(bytes / 1024).toFixed(1)} KiB (${bytes.toLocaleString("en-US")} B)`,
    );
  }
  return u(
    `${(bytes / (1024 * 1024)).toFixed(1)} MiB（${bytes.toLocaleString("zh-CN")} 字节）`,
    `${(bytes / (1024 * 1024)).toFixed(1)} MiB (${bytes.toLocaleString("en-US")} B)`,
  );
}

/** 本地时间（含时区标注）；原始 UTC 毫秒在诊断中保留。 */
export function formatLocalTime(millis: number | null): string {
  if (millis === null) {
    return u("未知", "Unknown");
  }
  const date = new Date(millis);
  if (Number.isNaN(date.getTime())) {
    return u("未知", "Unknown");
  }
  const pad = (value: number) => String(value).padStart(2, "0");
  const zoneMinutes = -date.getTimezoneOffset();
  const sign = zoneMinutes >= 0 ? "+" : "-";
  const zone = `UTC${sign}${pad(Math.floor(Math.abs(zoneMinutes) / 60))}:${pad(Math.abs(zoneMinutes) % 60)}`;
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())} ${pad(date.getHours())}:${pad(date.getMinutes())}:${pad(date.getSeconds())}（${zone}）`;
}

/** RFC 3339（契约 snapshot_time）→ 本地时间文本。 */
export function formatSnapshotTime(iso: string | null): string {
  if (!iso) {
    return u("未知", "Unknown");
  }
  const millis = Date.parse(iso);
  return Number.isNaN(millis) ? iso : formatLocalTime(millis);
}

/** 短 ID：保留前缀 0x 与末 4 位，便于区分同名条目；完整 ID 保留在详情/证据。 */
export function shortId(id: string): string {
  if (id.length <= 8) {
    return id;
  }
  return `${id.slice(0, 4)}…${id.slice(-4)}`;
}

/** 路径中的文件名（同时接受 Windows 与 POSIX 分隔符；空路径返回原值）。 */
export function fileNameOf(path: string): string {
  const parts = path.split(/[\\/]/);
  return parts[parts.length - 1] || path;
}

/** 计数展示：`0/8`；目标未知时只显示数值。 */
export function formatCount(value: number | null, target: number | null): string {
  if (value === null) {
    return target === null ? u("未知", "Unknown") : u(`未知/${target}`, `Unknown/${target}`);
  }
  return target === null ? String(value) : `${value}/${target}`;
}

/** 计数单位（中文量词 / 英文复数）。 */
export function countLabel(count: number): string {
  return u(`${count} 项`, `${count} ${count === 1 ? "item" : "items"}`);
}
