// 界面语言（2026-10-20）：中文 / English / 中英文对照。
//
// 设计边界：
// - 默认中文；偏好随 SX-015 的界面偏好一起持久化；
// - 只负责“显示哪一段文字”，不翻译后端返回的自由文本（诊断消息、限制说明等
//   由 Rust/模拟数据提供，保留原文，便于与导出产物对照）；
// - 图鉴、区域、子关卡、获取说明等带 zh/en 字段的数据在渲染/计划构建时按当前
//   语言取值；语言变化会让依赖它的计算属性重新求值；
// - “对照”用 `中文 / English` 单行并列，便于在窄窗中保持逐行对应。
import { ref } from "vue";

export type LanguagePreference = "zh" | "en" | "both";

const language = ref<LanguagePreference>("zh");

/** 只读引用：需要让计算属性随语言变化重新求值时读取它。 */
export const languageRef = language;

export function currentLanguage(): LanguagePreference {
  return language.value;
}

export function isEnglish(): boolean {
  return language.value === "en";
}

export function isBilingual(): boolean {
  return language.value === "both";
}

/** 设置语言并同步 <html lang>（SSR/测试环境没有 document 时静默）。 */
export function setLanguage(preference: LanguagePreference): void {
  language.value = preference;
  if (typeof document !== "undefined" && document.documentElement) {
    document.documentElement.lang = preference === "en" ? "en" : "zh-CN";
    document.documentElement.setAttribute("data-language", preference);
  }
}

/**
 * 静态界面文案：
 * - zh：中文；
 * - en：英文（英文缺失时回落中文）；
 * - both：`中文 / English`（两种语言相同或英文缺失时只显示中文）。
 */
export function t(zh: string, en: string): string {
  switch (language.value) {
    case "en":
      return en.length > 0 ? en : zh;
    case "both":
      if (en.length === 0 || en === zh) {
        return zh;
      }
      return `${zh} / ${en}`;
    default:
      return zh.length > 0 ? zh : en;
  }
}

/**
 * 数据名称（游戏条目、区域、子关卡）：中英文各自可能缺失；
 * 优先当前语言，缺失时回退另一种语言，最后回退 `fallback`。
 */
export function localizedName(
  zh: string | null | undefined,
  en: string | null | undefined,
  fallback = "",
): string {
  const zhText = zh && zh.trim().length > 0 ? zh : null;
  const enText = en && en.trim().length > 0 ? en : null;
  switch (language.value) {
    case "en":
      return enText ?? zhText ?? fallback;
    case "both":
      if (zhText && enText && zhText !== enText) {
        return `${zhText} / ${enText}`;
      }
      return zhText ?? enText ?? fallback;
    default:
      return zhText ?? enText ?? fallback;
  }
}

/**
 * 带插值的静态文案：模板中的 `{name}` 由 `values` 替换。
 * 用于把已经按当前语言显示的动态内容（区域名、条目名）嵌入句子，
 * 避免“对照”模式把同一段动态内容在中英文两侧各拼一次。
 * 对照模式使用中文模板 + 已本地化的动态内容。
 */
export function tFormat(
  zhTemplate: string,
  enTemplate: string,
  values: Record<string, string | number>,
): string {
  const template = language.value === "en" ? enTemplate : zhTemplate;
  return template.replace(/\{(\w+)\}/g, (_match, key: string) =>
    key in values ? String(values[key]) : `{${key}}`,
  );
}

/** 标签后的冒号（英文用半角 + 空格，中文用全角）。 */
export function labelColon(): string {
  return language.value === "en" ? ": " : "：";
}

/** 列表连接符：中文用顿号，英文用逗号。 */
export function listSeparator(): string {
  return language.value === "zh" ? "、" : ", ";
}

export function joinList(values: (string | null | undefined)[]): string {
  return values.filter((value): value is string => Boolean(value && value.length > 0)).join(listSeparator());
}
