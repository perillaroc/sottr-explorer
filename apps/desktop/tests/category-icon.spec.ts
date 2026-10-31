// 类别图标映射（2026-10-31 地图查询与详情增强·阶段二）：
// 图鉴全部类别都有专属图标；未知类别回退 MapPin（不猜、不随机分配）。
import { describe, expect, it } from "vitest";
import { MapPin } from "@lucide/vue";
import { categoryIcon, fallbackCategoryIcon, iconCategoryIds } from "../src/app/category-icon";
import { productionAtlas } from "../src/data/atlas";

describe("类别图标", () => {
  it("图鉴的 12 个类别都有专属图标映射", () => {
    const atlasCategories = productionAtlas.categories.map((category) => category.id).sort();
    expect(iconCategoryIds().sort()).toEqual(atlasCategories);
    for (const id of atlasCategories) {
      expect(categoryIcon(id)).not.toBe(fallbackCategoryIcon);
    }
  });

  it("未知类别回退 MapPin", () => {
    expect(categoryIcon("no_such_category")).toBe(MapPin);
    expect(categoryIcon(null)).toBe(MapPin);
    expect(fallbackCategoryIcon).toBe(MapPin);
  });
});
