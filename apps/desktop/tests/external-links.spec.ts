// 外部链接构造（2026-10-17 详情重设计阶段三）：搜索地址拼接。
import { describe, expect, it } from "vitest";
import {
  SEARCH_ENGINES,
  buildSearchUrl,
  textSearchQuery,
  videoSearchQuery,
} from "../src/app/external-links";
import type { PlanItem } from "../src/data/atlas";

function item(overrides: Partial<PlanItem> = {}): PlanItem {
  return {
    id: "0x0F28B6EC",
    display_name: "圣胡安贮藏点 #1",
    name_en: "San Juan Cache 1",
    status: "not_collected",
    is_dlc: false,
    required_for_100: true,
    acquisition_text: "类别规则原文",
    acquisition_source: "category_rule",
    location_hint: "圣胡安 · 贮藏点",
    ...overrides,
  };
}

describe("外部链接：搜索地址", () => {
  it("三个引擎各自把编码后的查询词填进占位符", () => {
    const query = "圣胡安 古墓丽影暗影 攻略";
    const encoded = encodeURIComponent(query);
    expect(buildSearchUrl("bing", query)).toBe(`https://www.bing.com/search?q=${encoded}`);
    expect(buildSearchUrl("google", query)).toBe(`https://www.google.com/search?q=${encoded}`);
    expect(buildSearchUrl("baidu", query)).toBe(`https://www.baidu.com/s?wd=${encoded}`);
    expect(Object.keys(SEARCH_ENGINES).sort()).toEqual(["baidu", "bing", "google"]);
  });

  it("查询词整体编码：空格与保留字符不会拼出非法地址", () => {
    expect(buildSearchUrl("bing", "a b&c")).toBe(
      "https://www.bing.com/search?q=a%20b%26c",
    );
    expect(buildSearchUrl("bing", "a b&c")).not.toContain(" ");
  });

  it("查询词带条目名与区域名；缺英文名时不留下空档", () => {
    expect(textSearchQuery(item(), "圣胡安")).toBe(
      "圣胡安贮藏点 #1 San Juan Cache 1 圣胡安 古墓丽影暗影 攻略",
    );
    expect(videoSearchQuery(item(), "圣胡安")).toBe(
      "圣胡安贮藏点 #1 圣胡安 古墓丽影暗影 视频攻略",
    );
    expect(textSearchQuery(item({ name_en: undefined }), "圣胡安")).toBe(
      "圣胡安贮藏点 #1 圣胡安 古墓丽影暗影 攻略",
    );
  });
});
