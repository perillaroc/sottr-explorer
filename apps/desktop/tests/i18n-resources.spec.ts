// P4-1 文案资源化：资源完整性、缺失回退与对照模式语义。
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { en, LANGUAGE_OPTIONS, LOCALES, localeResources, zhCN } from "../src/app/locales";
import {
  clearMissingTranslations,
  missingTranslations,
  setLanguage,
  t,
  tFormat,
  tFormatPair,
} from "../src/app/i18n";

describe("文案资源", () => {
  beforeEach(() => {
    setLanguage("zh");
    clearMissingTranslations();
  });
  afterEach(() => {
    setLanguage("zh");
    clearMissingTranslations();
  });

  it("英文覆盖全部中文键（新增键时必须补翻译）", () => {
    const missing = Object.keys(zhCN).filter((key) => !(key in en));
    expect(missing).toEqual([]);
  });

  it("键名为 命名空间.语义名，且不带中文", () => {
    for (const key of Object.keys(zhCN)) {
      expect(key).toMatch(/^[a-z][a-zA-Z0-9]*\.[A-Za-z0-9_-]+$/);
    }
  });

  it("注册表包含全部语言与对照项", () => {
    expect(Object.keys(localeResources)).toEqual(["zh", "en"]);
    expect(LOCALES.map((entry) => entry.code)).toEqual(["zh", "en"]);
    expect(LANGUAGE_OPTIONS.map((option) => option.value)).toEqual(["zh", "en", "both"]);
  });

  it("英文缺失时回退中文并记录，英文恢复后不再记录", () => {
    const saved = en["common.cancel"];
    delete (en as Record<string, string | undefined>)["common.cancel"];
    try {
      setLanguage("en");
      expect(t("common.cancel")).toBe(zhCN["common.cancel"]);
      expect(missingTranslations()).toContain("en:common.cancel");
    } finally {
      (en as Record<string, string | undefined>)["common.cancel"] = saved;
    }
    clearMissingTranslations();
    expect(t("common.cancel")).toBe(saved);
    expect(missingTranslations()).toEqual([]);
  });

  it("对照模式：普通文案并列，模板按各自语义渲染", () => {
    setLanguage("both");
    expect(t("common.cancel")).toBe(`${zhCN["common.cancel"]} / ${en["common.cancel"]}`);

    // tFormat 只渲染中文模板（动态值本身已本地化，避免重复）。
    expect(tFormat("app.code", { code: "X" })).toBe("原因码：X");
    // tFormatPair 两种模板并列（动态值不随语言变化）。
    expect(tFormatPair("app.details", { v0: "X" })).toBe("详情 · X / Details · X");
  });

  it("英文模式：模板两种函数都取英文模板", () => {
    setLanguage("en");
    expect(tFormat("app.code", { code: "X" })).toBe("Code: X");
    expect(tFormatPair("app.details", { v0: "X" })).toBe("Details · X");
  });

  it("占位符缺失时保留原样，便于发现模板错误", () => {
    setLanguage("zh");
    expect(tFormat("app.code", {})).toBe("原因码：{code}");
  });
});
