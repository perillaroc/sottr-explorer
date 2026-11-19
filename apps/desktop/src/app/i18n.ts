// 界面语言（2026-10-20；2026-11-19 P4-1 资源化）：中文 / English / 中英文对照。
//
// 设计边界：
// - 静态界面文案放在 `locales/zh-CN.ts` 与 `locales/en.ts`，调用点只写 MessageKey；
//   英文缺失时回退中文，并记录到 `missingTranslations()`；
// - 默认中文；偏好随 SX-015 的界面偏好一起持久化；
// - 不翻译后端返回的自由文本（诊断消息、限制说明等由 Rust/模拟数据提供，保留原文，
//   便于与导出产物对照）；
// - 图鉴、区域、子关卡、获取说明等带 zh/en 字段的数据在渲染/计划构建时按当前
//   语言取值（`localizedName`）；语言变化会让依赖它的计算属性重新求值；
// - “对照”用 `中文 / English` 单行并列，便于在窄窗中保持逐行对应。
import { ref } from "vue";
import {
  FALLBACK_LOCALE,
  LOCALES,
  localeResources,
  type LocaleCode,
  type LanguagePreference,
  type MessageKey,
} from "./locales";

export type { LanguagePreference, MessageKey };

const language = ref<LanguagePreference>("zh");

/** 只读引用：需要让计算属性随语言变化重新求值时读取它。 */
export const languageRef = language;

/** 运行期记录的缺失翻译（语言代码 + 键），供诊断与测试使用。 */
const missing = new Set<string>();
const warned = new Set<string>();

function recordMissing(locale: LocaleCode, key: MessageKey): void {
  const id = `${locale}:${key}`;
  missing.add(id);
  if (!warned.has(id)) {
    warned.add(id);
    if (import.meta.env?.DEV) {
      console.warn(`[i18n] 缺少 ${locale} 文案：${key}（已回退 ${FALLBACK_LOCALE}）`);
    }
  }
}

/** 当前进程内出现过的缺失翻译，形如 `en:common.cancel`。 */
export function missingTranslations(): string[] {
  return [...missing];
}

/** 仅供测试重置记录。 */
export function clearMissingTranslations(): void {
  missing.clear();
  warned.clear();
}

function textFor(locale: LocaleCode, key: MessageKey): string | undefined {
  const text = localeResources[locale][key];
  return text && text.length > 0 ? text : undefined;
}

/** 取键的文案：缺失时回退中文；两者都没有时返回键名（便于发现）。 */
function resolve(locale: LocaleCode, key: MessageKey): string {
  const text = textFor(locale, key);
  if (text !== undefined) {
    return text;
  }
  const fallback = textFor(FALLBACK_LOCALE, key);
  if (fallback !== undefined) {
    if (locale !== FALLBACK_LOCALE) {
      recordMissing(locale, key);
    }
    return fallback;
  }
  recordMissing(locale, key);
  return key;
}

function render(template: string, values: Record<string, string | number>): string {
  return template.replace(/\{(\w+)\}/g, (_match, key: string) =>
    key in values ? String(values[key]) : `{${key}}`,
  );
}

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
    const locale = LOCALES.find((entry) => entry.code === preference);
    document.documentElement.lang = locale?.htmlLang ?? FALLBACK_LOCALE;
    document.documentElement.setAttribute("data-language", preference);
  }
}

/**
 * 静态界面文案：
 * - zh：中文；
 * - en：英文（英文缺失时回落中文并记录）；
 * - both：`中文 / English`（两种语言相同或英文缺失时只显示中文）。
 */
export function t(key: MessageKey): string {
  const preference = language.value;
  if (preference === "both") {
    const zh = resolve("zh", key);
    const en = textFor("en", key);
    if (en === undefined) {
      recordMissing("en", key);
      return zh;
    }
    if (en === zh) {
      return zh;
    }
    return `${zh} / ${en}`;
  }
  return resolve(preference, key);
}

/**
 * 带插值的静态文案：模板中的 `{name}` 由 `values` 替换。
 * 用于把已经按当前语言显示的动态内容（区域名、条目名）嵌入句子，
 * 避免“对照”模式把同一段动态内容在中英文两侧各拼一次。
 * 对照模式使用中文模板 + 已本地化的动态内容。
 */
export function tFormat(key: MessageKey, values: Record<string, string | number>): string {
  const template = language.value === "en" ? resolve("en", key) : resolve("zh", key);
  return render(template, values);
}

/**
 * 带插值、且对照模式需要两种模板并列的静态文案。
 * 用于动态值本身不随语言变化的句子（路径、计数、错误消息）。
 */
export function tFormatPair(key: MessageKey, values: Record<string, string | number>): string {
  const preference = language.value;
  if (preference === "en") {
    return render(resolve("en", key), values);
  }
  const zhTemplate = resolve("zh", key);
  if (preference === "both") {
    const enTemplate = textFor("en", key);
    if (enTemplate === undefined) {
      recordMissing("en", key);
      return render(zhTemplate, values);
    }
    if (enTemplate === zhTemplate) {
      return render(zhTemplate, values);
    }
    return `${render(zhTemplate, values)} / ${render(enTemplate, values)}`;
  }
  return render(resolve(preference, key), values);
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
