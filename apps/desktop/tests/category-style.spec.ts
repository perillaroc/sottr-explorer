// 类别着色映射（2026-10-17 详情重设计阶段五）：映射与图鉴类别集合、与 styles.css 保持一致。
import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { categoryClass, categoryDotClass, styledCategoryIds } from "../src/app/category-style";
import { productionAtlas } from "../src/data/atlas";

// 直接读源文件（不经 Vite 转换）：本用例核对的就是样式源里的令牌与工具类。
// vitest 的工作目录是应用根（apps/desktop）。
const styles = readFileSync("src/styles.css", "utf8");

describe("类别着色映射", () => {
  it("与图鉴类别集合一一对应（不多不少，无拼写漂移）", () => {
    expect(styledCategoryIds().sort()).toEqual(productionAtlas.categories.map((entry) => entry.id).sort());
  });

  it("已知类别给出文字与圆点类名；未知或空值不着色", () => {
    expect(categoryClass("artifacts")).toBe("cat-text-artifacts");
    expect(categoryDotClass("artifacts")).toBe("cat-dot-artifacts");
    expect(categoryClass("no_such_category")).toBe("");
    expect(categoryDotClass("no_such_category")).toBe("");
    expect(categoryClass(null)).toBe("");
    expect(categoryDotClass(undefined)).toBe("");
  });

  it("styles.css 为每个类别定义了明暗两套令牌与两个工具类", () => {
    for (const id of styledCategoryIds()) {
      // 令牌必须出现两次：浅色与深色主题各一套。
      expect(styles.split(`--cat-${id}:`).length - 1, `--cat-${id} 应有明暗两套`).toBe(2);
      expect(styles).toContain(`.cat-text-${id} {`);
      expect(styles).toContain(`.cat-dot-${id} {`);
    }
  });
});
