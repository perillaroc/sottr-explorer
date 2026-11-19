// 语言注册表：新增语言时只需
//   1) 复制 zh-CN.ts 翻译成 <locale>.ts 并导出同名消息表；
//   2) 在 localeResources 与 LOCALES 各加一行。
// 调用点只认 MessageKey，不会随语言数量变化。
import { zhCN, type MessageKey } from "./zh-CN";
import { en } from "./en";

export type { MessageKey };
export { zhCN, en };

/** 语言代码 → 消息表。回退语言固定为中文（键的权威来源）。 */
export const localeResources = { zh: zhCN, en } as const;

export type LocaleCode = keyof typeof localeResources;

export const FALLBACK_LOCALE: LocaleCode = "zh";

export interface LocaleMeta {
  code: LocaleCode;
  /** 写入 <html lang> 的 BCP 47 值。 */
  htmlLang: string;
  /** 语言自称（如 “English”）；为空时用 labelKey。 */
  label?: string;
  labelKey?: MessageKey;
}

export const LOCALES: LocaleMeta[] = [
  { code: "zh", htmlLang: "zh-CN", labelKey: "settingsPage.chinese" },
  { code: "en", htmlLang: "en", label: "English" },
];

/** 界面语言偏好：具体语言，或中英对照。 */
export type LanguagePreference = LocaleCode | "both";

export interface LanguageOption {
  value: LanguagePreference;
  /** 语言自称（不随界面语言变化）。 */
  label?: string;
  labelKey?: MessageKey;
  hintKey: MessageKey;
}

export const LANGUAGE_OPTIONS: LanguageOption[] = [
  { value: "zh", labelKey: "settingsPage.chinese", hintKey: "settingsPage.show-chinese-only" },
  { value: "en", label: "English", hintKey: "settingsPage.show-english-only" },
  { value: "both", labelKey: "settingsPage.bilingual", hintKey: "settingsPage.show-chinese-and-english-side-by-side" },
];
